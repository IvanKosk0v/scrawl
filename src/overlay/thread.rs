//! The overlay's render thread (M7.4). The event loop only queues events: [`OverlayHandle::send`]
//! never waits for drawing. Events queued while a frame is drawn are merged by [`coalesce`], so the
//! trail catches up after a slow frame and the queue never holds more than three events.

use std::sync::{Arc, Condvar, Mutex};

use super::Overlay;
use crate::error::Error;

/// One overlay call, as queued by the event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayEvent {
    /// [`Overlay::begin`].
    Begin,
    /// [`Overlay::motion`] with this relative delta.
    Motion(i32, i32),
    /// [`Overlay::end`].
    End,
}

/// Queues `ev`, merging it into what is still waiting: `Begin` drops everything before it (it
/// restarts the trail), a `Motion` after a `Motion` adds to it (saturating). The queue thus holds at
/// most `[Motion, End, Begin]`-like runs of length 3.
pub fn coalesce(queue: &mut Vec<OverlayEvent>, ev: OverlayEvent) {
    match (queue.last_mut(), ev) {
        (_, OverlayEvent::Begin) => {
            queue.clear();
            queue.push(ev);
        }
        (Some(OverlayEvent::Motion(x, y)), OverlayEvent::Motion(dx, dy)) => {
            *x = x.saturating_add(dx);
            *y = y.saturating_add(dy);
        }
        // `End` then `End` (or a stray one) is a no-op on every backend: keep only one.
        (Some(OverlayEvent::End), OverlayEvent::End) => {}
        _ => {
            // Without a Begin in between, at most [Motion, End, Motion]: keep the stale prefix short.
            if queue.len() >= 3 {
                queue.remove(0);
            }
            queue.push(ev);
        }
    }
}

#[derive(Debug, Default)]
struct Shared {
    queue: Vec<OverlayEvent>,
    /// Set when the handle is dropped: the thread stops.
    closed: bool,
    /// Cleared when the thread exits (creation or render error, or `closed`).
    running: bool,
}

type Sync = Arc<(Mutex<Shared>, Condvar)>;

/// The event loop's side of the render thread. Dropping it stops the thread (without waiting).
#[derive(Debug)]
pub struct OverlayHandle {
    shared: Sync,
}

impl OverlayHandle {
    /// Queues `ev` for the render thread; never waits for rendering. After the thread has stopped
    /// (e.g. no layer shell) it only updates the bounded queue.
    pub fn send(&self, ev: OverlayEvent) {
        let (lock, cvar) = &*self.shared;
        // A poisoned lock means the render thread panicked, and the panic hook already exits.
        if let Ok(mut s) = lock.lock() {
            coalesce(&mut s.queue, ev);
            cvar.notify_one();
        }
    }

    /// True until the render thread has exited.
    pub fn is_running(&self) -> bool {
        self.shared.0.lock().map(|s| s.running).unwrap_or(false)
    }
}

impl Drop for OverlayHandle {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.shared;
        if let Ok(mut s) = lock.lock() {
            s.closed = true;
        }
        cvar.notify_one();
    }
}

/// Starts the render thread. The overlay is created there by `make` (Wayland blocks until its first
/// configure). A creation or render error is printed once as `overlay disabled: ...` and stops the
/// thread; the daemon keeps running without a trail.
pub fn spawn<F>(make: F) -> OverlayHandle
where
    F: FnOnce() -> Result<Box<dyn Overlay>, Error> + Send + 'static,
{
    let shared: Sync = Arc::new((
        Mutex::new(Shared {
            running: true,
            ..Shared::default()
        }),
        Condvar::new(),
    ));
    let theirs = Arc::clone(&shared);
    let started = std::thread::Builder::new()
        .name("strokebind-overlay".into())
        .spawn(move || {
            if let Err(e) = make().and_then(|o| render_loop(o, &theirs)) {
                eprintln!("overlay disabled: {e}");
            }
            if let Ok(mut s) = theirs.0.lock() {
                s.running = false;
            }
        });
    if let Err(e) = started {
        eprintln!("overlay disabled: cannot start its thread: {e}");
        if let Ok(mut s) = shared.0.lock() {
            s.running = false;
        }
    }
    OverlayHandle { shared }
}

/// Applies queued events until the handle is dropped (`Ok`) or the overlay fails.
fn render_loop(mut overlay: Box<dyn Overlay>, shared: &Sync) -> Result<(), Error> {
    let (lock, cvar) = &**shared;
    loop {
        let batch = {
            let mut s = lock
                .lock()
                .map_err(|_| Error::Overlay("overlay queue poisoned".into()))?;
            while s.queue.is_empty() && !s.closed {
                s = cvar
                    .wait(s)
                    .map_err(|_| Error::Overlay("overlay queue poisoned".into()))?;
            }
            if s.closed {
                return Ok(());
            }
            std::mem::take(&mut s.queue)
        };
        for ev in batch {
            match ev {
                OverlayEvent::Begin => overlay.begin()?,
                OverlayEvent::Motion(dx, dy) => overlay.motion(dx, dy)?,
                OverlayEvent::End => overlay.end()?,
            }
        }
    }
}
