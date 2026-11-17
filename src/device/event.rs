//! Device-independent input events.

use crate::config::Trigger;

/// A simplified input event, one per kernel report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// A trigger-capable button changed state.
    Button {
        /// Which button.
        button: Trigger,
        /// True on press, false on release.
        pressed: bool,
    },
    /// Relative pointer motion in device units.
    Motion {
        /// Horizontal delta.
        dx: i32,
        /// Vertical delta (positive is down).
        dy: i32,
    },
}
