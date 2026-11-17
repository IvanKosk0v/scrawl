//! M0.1: the crate exposes the planned module layout.

use strokebind::config::Config;
use strokebind::error::Error;

#[test]
fn layout_modules_exist() {
    let _: Option<Config> = None;
    let _ = (
        strokebind::recognizer::MODULE,
        strokebind::device::list_mice_in as fn(&std::path::Path) -> _,
        strokebind::sys::MODULE,
    );
}

#[test]
fn error_displays_message() {
    let e = Error::Unsupported("grab");
    assert_eq!(e.to_string(), "unsupported: grab");
}
