use std::fs::File;
use std::io;
use std::io::Write;
use std::thread;
use std::thread::JoinHandle;

use crossbeam::Receiver;

pub fn start(input: Receiver<String>) -> JoinHandle<io::Result<()>> {
    thread::spawn(move || {
        let mut file = File::create("console")?;
        for command in input {
            writeln!(file, "{}", command)?;
            file.flush()?;
        }
        Ok(())
    })
}
