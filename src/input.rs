use rustbox::Event;
use crossbeam::Sender;
use crossbeam::Receiver;
use std::sync::Arc;
use rustbox::RustBox;
use std::thread;
use std::thread::JoinHandle;

pub fn start(rb: Arc<RustBox>, input: Receiver<()>, output: Sender<Event>) -> JoinHandle<()> {
    thread::spawn(move || {
        let rb = rb;
        loop {
            let event = rb.poll_event(false);
            output.send(event.unwrap());

            if input.recv().is_none() {
                break;
            }
        }
    })
}
