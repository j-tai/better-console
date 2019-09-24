use std::io;
use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::thread;
use std::thread::JoinHandle;

use crossbeam::Sender;

use crate::output::Line;

pub fn spawn_tail() -> io::Result<Child> {
    let mut cmd = Command::new("tail");
    cmd.arg("-n0");
    cmd.arg("-F");
    cmd.arg("logs/latest.log");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    cmd.spawn()
}

pub fn start(stdout: ChildStdout, output: Sender<Line>) -> JoinHandle<io::Result<()>> {
    thread::spawn(move || {
        let stdout = BufReader::new(stdout);

        for line in stdout.lines() {
            output.send(Line::Log(line?.replace('\t', "    "))).unwrap();
        }

        Ok(())
    })
}
