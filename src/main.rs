#[macro_use]
extern crate crossbeam;
extern crate flate2;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate rustbox;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;

use std::fs::File;
use std::io::{ErrorKind, Read};
use std::process;
use std::sync::Arc;

use crossbeam::channel;
use rustbox::{InitOptions, OutputMode, RustBox};

use config::Config;

mod command;
mod config;
mod history;
mod input;
mod logs;
mod output;

fn main() {
    // Load configuration:
    let conf = match File::open("better-console.toml") {
        Ok(mut f) => {
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            toml::from_str::<Config>(&s).unwrap_or_else(|e| {
                eprintln!("failed to parse configuration: {}", e);
                process::exit(2);
            })
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            // File not found, so we'll use the default configuration.
            Config::default()
        }
        Err(e) => {
            eprintln!("failed to read configuration: {}", e);
            process::exit(2);
        }
    };
    let conf = Arc::new(conf);

    // Initialize RustBox:
    let rb = RustBox::init(InitOptions {
        buffer_stderr: false,
        output_mode: OutputMode::EightBit,
        ..Default::default()
    })
    .unwrap_or_else(|e| {
        eprintln!("failed to init terminal: {}", e);
        process::exit(2);
    });
    let rb = Arc::new(rb);

    // Initialize channels and threads:
    // Quit signal for input thread
    let (send_iq, recv_iq) = channel::bounded(0);
    // Quit signal for history thread
    let (send_hq, recv_hq) = channel::bounded(0);

    // Logs thread -- sends new incoming logs to the output thread
    let mut tail = logs::spawn_tail().unwrap_or_else(|e| {
        eprintln!("failed to spawn tail: {}", e);
        process::exit(2)
    });
    let (send_l, recv_l) = channel::bounded(16);
    let logs = logs::start(tail.stdout.take().unwrap(), send_l);

    // History thread -- sends old logs to the output thread when requested
    let (send_h, recv_h) = channel::bounded(16);
    let history = history::start(recv_hq, send_h);

    // Input thread -- forwards user input to the output thread
    let (send_i, recv_i) = channel::bounded(0);
    let input = input::start(rb.clone(), recv_iq, send_i);

    // Command thread -- sends commands to the server when requested
    let (send_c, recv_c) = channel::bounded(16);
    let command = command::start(recv_c);

    // Run the output ("main") thread.
    output::run(conf, rb, recv_h, recv_l, recv_i, send_c, send_iq);

    // Cleanup:
    // Input thread as the output thread has commanded.
    input.join().unwrap();
    // Command thread should terminate automatically once the output thread exits.
    command.join().unwrap().unwrap();
    // Drop the history thread sender so that the history thread terminates.
    drop(send_hq);
    history.join().unwrap().unwrap();
    // Kill the tail process so that the logs thread terminates.
    tail.kill().unwrap();
    logs.join().unwrap().unwrap();
}
