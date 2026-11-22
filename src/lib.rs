//! scrawl: mouse-stroke gestures on Linux via evdev/uinput.

pub mod actions;
pub mod backend;
pub mod check;
pub mod cli;
pub mod config;
pub mod device;
pub mod error;
pub mod focus;
pub mod hyprland;
pub mod overlay;
pub mod passthrough;
pub mod recognizer;
pub mod reload;
pub mod stroke_loop;
pub mod sway;
pub mod sys;
pub mod window;
pub mod x11;
