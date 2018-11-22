use crossbeam::Sender;
use std::thread::JoinHandle;
use std::io;
use std::thread;
use std::process::Command;
use std::process::Stdio;
use std::io::BufReader;
use std::io::BufRead;
use std::process::Child;
use std::process::ChildStdout;
use output::Line;

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
            output.send(Line::Log(line?.replace('\t', "    ")));
        }

        Ok(())
    })
}
