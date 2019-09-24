use std::borrow::Cow;
use std::collections::VecDeque;
use std::mem;
use std::sync::Arc;

use crossbeam::{Receiver, Sender};
use regex::Regex;
use rustbox::{Event, Key, RustBox};

use config::{Color, Config};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Line {
    Log(String),
    Header(String),
}

struct Text<'a> {
    text: Cow<'a, str>,
    color: Color,
}

impl<'a> Text<'a> {
    fn new(text: Cow<'a, str>, color: Color) -> Text {
        Text { text, color }
    }

    fn normal(text: Cow<'a, str>) -> Text {
        Text {
            text,
            color: Color::default(),
        }
    }
}

struct Console {
    config: Arc<Config>,
    buffer: VecDeque<Line>,
    input: String,
    scroll: isize,
    hscroll: isize,
    width: isize,
    height: isize,
    exit: bool,
    rb: Arc<RustBox>,
    send_c: Sender<String>,
    send_i: Sender<()>,
}

impl Console {
    fn mainloop(
        &mut self,
        recv_h: Receiver<Line>,
        recv_l: Receiver<Line>,
        recv_i: Receiver<Event>,
    ) {
        self.width = self.rb.width() as isize;
        self.height = self.rb.height() as isize;

        self.collect_logs(&recv_h, &recv_l);

        self.draw_all();
        while !self.exit {
            self.rb.present();
            select! {
                recv(recv_l, log) => {
                    if self.scroll == self.max_scroll() {
                        self.scroll += 1;
                    }
                    self.buffer.push_back(log.unwrap());
                    self.draw_logs();
                }
                recv(recv_i, event) => {
                    self.process_event(&recv_h, event.unwrap());
                }
            }
        }
    }

    fn collect_logs(&mut self, recv_h: &Receiver<Line>, recv_l: &Receiver<Line>) {
        // Collect enough logs
        'c: while (self.buffer.len() as isize) < self.height - 2 {
            select! {
                recv(recv_h, log) => {
                    if let Some(log) = log {
                        self.buffer.push_front(log);
                    } else {
                        return;
                    }
                }
                recv(recv_l, log) => {
                    self.buffer.push_back(log.unwrap());
                }
            }
        }
    }

    fn process_event(&mut self, recv_h: &Receiver<Line>, event: Event) {
        let height = self.height;
        let vert_move = self.config.vertical_move;
        let horiz_move = self.config.horizontal_move;
        match event {
            Event::ResizeEvent(w, h) => {
                self.width = w as isize;
                self.height = h as isize;
                self.draw_all();
            }
            Event::KeyEvent(key) => match key {
                Key::Ctrl('q') => {
                    self.exit = true;
                    return;
                }
                Key::Up => self.scroll(recv_h, -vert_move),
                Key::Down => self.scroll(recv_h, vert_move),
                Key::Left => self.scroll_h(-horiz_move),
                Key::Right => self.scroll_h(horiz_move),
                Key::PageUp => self.scroll(recv_h, -height / 2),
                Key::PageDown => self.scroll(recv_h, height / 2),
                Key::End => self.scroll_to_end(),
                Key::Char(c) => {
                    self.input.push(c);
                    self.draw_input();
                }
                Key::Backspace => {
                    self.input.pop();
                    self.draw_input();
                }
                Key::Enter => {
                    let mut command = String::new();
                    mem::swap(&mut command, &mut self.input);
                    self.send_c.send(command);
                    self.draw_input();
                }
                _ => (),
            },
            _ => (),
        }
        // Tell the input thread to keep going.
        self.send_i.send(());
    }

    /// Get the maximum value for `scroll`.
    fn max_scroll(&self) -> isize {
        let h = self.height - 2;
        let l = self.buffer.len() as isize;
        if l < h {
            0
        } else {
            l - h
        }
    }

    /// Add `delta` to `self.scroll` and redraw the logs. Fetches more old logs if necessary.
    fn scroll(&mut self, recv_h: &Receiver<Line>, delta: isize) {
        if delta == 0 {
            return;
        }
        if delta > 0 {
            let delta = delta;
            let max_scroll = self.max_scroll();
            if self.scroll + delta > max_scroll {
                self.scroll = max_scroll;
            } else {
                self.scroll += delta;
            }
        } else {
            let delta = -delta;
            if delta > self.scroll {
                let to_fetch = delta - self.scroll;
                for _ in 0..to_fetch {
                    let log = recv_h.recv();
                    if let Some(log) = log {
                        self.buffer.push_front(log);
                    } else {
                        // No more logs
                        break;
                    }
                }
                self.scroll = 0;
            } else {
                self.scroll -= delta;
            }
        }
        self.draw_logs();
    }

    /// Add `delta` to `self.hscroll` and redraw the logs if necessary.
    fn scroll_h(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }
        if delta > 0 {
            let delta = delta;
            self.hscroll += delta;
        } else {
            let delta = -delta;
            if self.hscroll == 0 {
                return;
            } else if self.hscroll < delta {
                self.hscroll = 0;
            } else {
                self.hscroll -= delta;
            }
        }
        self.draw_logs();
    }

    /// Scroll to the end of the logs, and redraw the logs.
    fn scroll_to_end(&mut self) {
        let max_scroll = self.max_scroll();
        if self.scroll != max_scroll {
            self.scroll = max_scroll;
            self.draw_logs();
        }
    }

    fn print(&self, mut x: isize, y: isize, mut s: &str, color: Color) {
        if x < 0 {
            if let Some((idx, _)) = s.char_indices().nth((-x) as usize) {
                s = &s[idx..];
            } else {
                return;
            }
            x = 0;
        }
        self.rb
            .print(x as usize, y as usize, color.sty, color.fg, color.bg, s);
    }

    fn print_line(&self, mut x: isize, y: isize, texts: Vec<Text>) {
        let left = x < 0;
        for text in texts {
            let s: &str = &*text.text;
            let len = s.chars().count() as isize;
            self.print(x, y, s, text.color);
            x += len;
        }

        let remaining = self.width - x;
        let right = remaining < 0;
        if remaining > 0 {
            let spaces = " ".repeat(remaining as usize);
            self.print(x, y, &spaces, Color::default());
        }

        if left {
            self.print(0, y, &self.config.trun_left, self.config.colors.truncate);
        }
        if right {
            let len = self.config.trun_right.chars().count() as isize;
            self.print(
                self.width - len,
                y,
                &self.config.trun_right,
                self.config.colors.truncate,
            );
        }
    }

    fn draw_all(&mut self) {
        self.draw_logs();
        self.draw_input();
        self.draw_status();
    }

    fn draw_logs(&mut self) {
        for i in 0..(self.height - 2) {
            if let Some(msg) = self.buffer.get((i + self.scroll) as usize) {
                match msg {
                    Line::Log(s) => {
                        let texts = self.format_log(s);
                        self.print_line(-(self.hscroll as isize), i, texts);
                    }
                    Line::Header(s) => {
                        // For headers, ignore horizontal scroll.
                        let width = (self.width - 6) as usize;
                        let output = format!(" --> {:.*}", width, s);
                        let texts = vec![Text::new(output.into(), self.config.colors.file_header)];
                        self.print_line(0, i, texts);
                    }
                }
            }
        }
    }

    fn format_log<'a>(&self, log: &'a str) -> Vec<Text<'a>> {
        lazy_static! {
            static ref REGEX: Regex =
                Regex::new(r"^\[(\d\d:\d\d:\d\d)] \[([^/]+)/([A-Z]+)]: (.*)$").unwrap();
        }
        if let Some(cap) = REGEX.captures(log) {
            vec![
                Text::new(cap[1].to_string().into(), self.config.colors.time),
                Text::normal(" ".into()),
                Text::new(
                    cap[3].to_string().into(),
                    match &cap[3] {
                        "INFO" => self.config.colors.info,
                        "WARN" => self.config.colors.warn,
                        "ERROR" => self.config.colors.error,
                        "SEVERE" => self.config.colors.severe,
                        "FATAL" => self.config.colors.fatal,
                        _ => self.config.colors.other,
                    },
                ),
                Text::normal(": ".into()),
                Text::new(cap[4].to_string().into(), self.config.colors.text),
            ]
        } else {
            vec![Text::new(log.into(), self.config.colors.text)]
        }
    }

    fn draw_input(&mut self) {
        let width = (self.width - 4) as usize;
        let output = format!(" > {:0$.*} ", width, self.input);
        self.print(0, self.height - 2, &output, self.config.colors.prompt);
        self.rb.set_cursor(
            self.input.chars().count() as isize + 3,
            self.height as isize - 2,
        )
    }

    fn draw_status(&mut self) {
        let width = (self.width - 2) as usize;
        let output = format!(" {:0$.*} ", width, self.config.default_status);
        self.print(0, self.height - 1, &output, self.config.colors.status);
    }
}

pub fn run(
    config: Arc<Config>,
    rustbox: Arc<RustBox>,
    recv_h: Receiver<Line>,
    recv_l: Receiver<Line>,
    recv_i: Receiver<Event>,
    send_c: Sender<String>,
    send_i: Sender<()>,
) {
    Console {
        config,
        buffer: VecDeque::new(),
        input: String::new(),
        scroll: 0,
        hscroll: 0,
        height: 0,
        width: 0,
        exit: false,
        rb: rustbox,
        send_c,
        send_i,
    }
    .mainloop(recv_h, recv_l, recv_i);
}
