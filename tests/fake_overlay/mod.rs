//! A fake overlay that records every call and can block in `motion` until released.
#![allow(dead_code, clippy::expect_used)]

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use strokebind::error::Error;
use strokebind::overlay::thread::OverlayEvent;
use strokebind::overlay::Overlay;

/// Records calls; while `gate` is set, the first `motion` waits for a message on it.
pub struct Fake {
    log: Arc<Mutex<Vec<OverlayEvent>>>,
    gate: Option<Receiver<()>>,
}

impl Overlay for Fake {
    fn begin(&mut self) -> Result<(), Error> {
        self.log.lock().expect("log").push(OverlayEvent::Begin);
        Ok(())
    }
    fn motion(&mut self, dx: i32, dy: i32) -> Result<(), Error> {
        if let Some(gate) = self.gate.take() {
            let _ = gate.recv();
        }
        self.log
            .lock()
            .expect("log")
            .push(OverlayEvent::Motion(dx, dy));
        Ok(())
    }
    fn end(&mut self) -> Result<(), Error> {
        self.log.lock().expect("log").push(OverlayEvent::End);
        Ok(())
    }
}

/// A fake whose first motion blocks until the returned sender sends (or is dropped).
pub fn blocking() -> (Fake, Sender<()>, Arc<Mutex<Vec<OverlayEvent>>>) {
    let (tx, rx) = channel();
    let log = Arc::new(Mutex::new(Vec::new()));
    let fake = Fake {
        log: Arc::clone(&log),
        gate: Some(rx),
    };
    (fake, tx, log)
}

/// Waits up to 5 s until the log ends with `End`, then returns a copy.
pub fn wait_for_end(log: &Arc<Mutex<Vec<OverlayEvent>>>) -> Vec<OverlayEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let seen = log.lock().expect("log").clone();
        if seen.last() == Some(&OverlayEvent::End) || Instant::now() > deadline {
            return seen;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Sum of all motion deltas in `log`.
pub fn total_motion(log: &[OverlayEvent]) -> (i32, i32) {
    log.iter().fold((0, 0), |(x, y), e| match e {
        OverlayEvent::Motion(dx, dy) => (x + dx, y + dy),
        _ => (x, y),
    })
}
