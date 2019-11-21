use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::thread;
use std::thread::JoinHandle;

use crossbeam::channel::select;
use crossbeam::{Receiver, Sender};
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use regex::Regex;

use crate::output::Line;

pub fn start(input: Receiver<()>, output: Sender<Line>) -> JoinHandle<io::Result<()>> {
    thread::spawn(move || {
        lazy_static! {
            static ref LOG_REGEX: Regex = Regex::new(r"^\d{4}-\d\d-\d\d-\d+\.log\.gz$").unwrap();
        }
        let mut filenames = Vec::new();
        for ent in fs::read_dir("logs")? {
            let ent = ent?;
            if let Some(s) = ent.file_name().to_str() {
                if LOG_REGEX.is_match(s) {
                    filenames.push(ent.path());
                }
            }
        }
        filenames.sort_unstable();

        let latest = File::open("logs/latest.log")?;
        let reader = BufReader::new(latest);
        let lines = reader.lines().collect::<io::Result<Vec<_>>>();
        let lines = lines?;
        for line in lines.into_iter().rev() {
            let log = Line::Log(line.replace('\t', "    "));
            if send(&input, &output, log) {
                return Ok(());
            }
        }
        let log = Line::Header("logs/latest.log".to_string());
        if send(&input, &output, log) {
            return Ok(());
        }

        for filename in filenames.into_iter().rev() {
            let raw_file = File::open(&filename)?;
            let decoder = GzDecoder::new(raw_file);
            let reader = BufReader::new(decoder);
            let lines = reader.lines().collect::<io::Result<Vec<_>>>();
            let lines = lines?;
            for line in lines.into_iter().rev() {
                let log = Line::Log(line.replace('\t', "    "));
                if send(&input, &output, log) {
                    return Ok(());
                }
            }
            let log = Line::Header(filename.to_string_lossy().into_owned());
            if send(&input, &output, log) {
                return Ok(());
            }
        }

        Ok(())
    })
}

fn send(input: &Receiver<()>, output: &Sender<Line>, line: Line) -> bool {
    loop {
        select! {
            recv(input) -> msg => if msg.is_err() { return true },
            send(output, line) -> _ => return false,
        }
    }
}
