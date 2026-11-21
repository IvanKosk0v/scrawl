//! Stroke trail overlay (M7): backend-independent trait, backend picker and shared renderer.
//!
//! Backends (X11 in M7.2, Wayland in M7.3) own a window and a pixel buffer; they draw it with
//! [`trail::render`] and convert the premultiplied RGBA result to their own pixel format.

use std::ffi::OsString;

use crate::backend::{display_server, DisplayServer};
use crate::error::Error;

pub mod trail;
pub mod wayland;
pub mod x11;

/// A surface that shows the trail of the stroke being drawn.
pub trait Overlay {
    /// Trigger pressed: show an empty trail. The backend picks the start point (cursor position on
    /// X11, screen center where no cursor position is available).
    fn begin(&mut self) -> Result<(), Error>;
    /// Relative pointer motion while the trigger is held: extend the trail.
    fn motion(&mut self, dx: i32, dy: i32) -> Result<(), Error>;
    /// Trigger released: hide the trail.
    fn end(&mut self) -> Result<(), Error>;
}

/// Which overlay backend this session can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayBackend {
    /// wlr-layer-shell surface (`WAYLAND_DISPLAY` set, even with XWayland's `DISPLAY`).
    Wayland,
    /// Override-redirect X11 window (only `DISPLAY` set).
    X11,
    /// No display server: no overlay.
    None,
}

/// The overlay backend from the session environment; same rule as [`display_server`].
pub fn overlay_backend(env: impl Fn(&str) -> Option<OsString>) -> OverlayBackend {
    match display_server(env) {
        DisplayServer::Wayland => OverlayBackend::Wayland,
        DisplayServer::X11 => OverlayBackend::X11,
        DisplayServer::None => OverlayBackend::None,
    }
}
