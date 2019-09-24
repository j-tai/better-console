use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use crossbeam::{Receiver, Sender};
use rustbox::{Event, RustBox};

pub fn start(rb: Arc<RustBox>, input: Receiver<()>, output: Sender<Event>) -> JoinHandle<()> {
    thread::spawn(move || {
        let rb = rb;
        loop {
            let event = rb.poll_event(false);
            output.send(event.unwrap()).unwrap();

            if input.recv().is_err() {
                break;
            }
        }
    })
}
