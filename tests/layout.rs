//! M0.1: the crate exposes the planned module layout.

use strokebind::config::Config;
use strokebind::error::Error;

#[test]
fn layout_modules_exist() {
    let _ = Config::default();
    let _ = (
        strokebind::recognizer::MODULE,
        strokebind::device::MODULE,
        strokebind::sys::MODULE,
    );
}

#[test]
fn error_displays_message() {
    let e = Error::Unsupported("grab");
    assert_eq!(e.to_string(), "unsupported: grab");
}
