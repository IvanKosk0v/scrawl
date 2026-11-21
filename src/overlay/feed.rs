//! Turns the raw event stream plus the engine's pending state into overlay events (pure).

use evdev::{EventSummary, InputEvent, RelativeAxisCode, SynchronizationCode};

use super::thread::OverlayEvent;

/// Accumulates REL_X/REL_Y of the current report; one `Motion` per report while the trigger is held.
#[derive(Debug, Default)]
pub struct TrailFeed {
    dx: i32,
    dy: i32,
}

impl TrailFeed {
    /// Feed one raw event with the engine's pending state before and after it. Pending changes only
    /// at `SYN_REPORT`: off->on = `Begin`, on->off = `End`, on->on = the report's motion (if any).
    /// Motion of the press and release reports is not drawn.
    pub fn push(&mut self, raw: &InputEvent, was: bool, now: bool) -> Option<OverlayEvent> {
        match raw.destructure() {
            EventSummary::RelativeAxis(_, RelativeAxisCode::REL_X, v) => {
                self.dx = self.dx.saturating_add(v);
                None
            }
            EventSummary::RelativeAxis(_, RelativeAxisCode::REL_Y, v) => {
                self.dy = self.dy.saturating_add(v);
                None
            }
            EventSummary::Synchronization(_, SynchronizationCode::SYN_REPORT, _) => {
                let (dx, dy) = std::mem::take(self).delta();
                match (was, now) {
                    (false, true) => Some(OverlayEvent::Begin),
                    (true, false) => Some(OverlayEvent::End),
                    (true, true) if (dx, dy) != (0, 0) => Some(OverlayEvent::Motion(dx, dy)),
                    _ => None,
                }
            }
            EventSummary::Synchronization(_, SynchronizationCode::SYN_DROPPED, _) => {
                *self = TrailFeed::default();
                None
            }
            _ => None,
        }
    }

    fn delta(&self) -> (i32, i32) {
        (self.dx, self.dy)
    }
}
