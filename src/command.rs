use crossbeam::Receiver;
use std::thread::JoinHandle;
use std::io;
use std::thread;
use std::fs::File;
use std::io::Write;

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
