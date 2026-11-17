//! strokebind: mouse-stroke gestures on Linux via evdev/uinput.

pub mod actions;
pub mod cli;
pub mod config;
pub mod device;
pub mod error;
pub mod passthrough;
pub mod recognizer;
pub mod stroke_loop;
pub mod sys;
