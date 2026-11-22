//! M0.1: the crate exposes the planned module layout.

use scrawl::config::Config;
use scrawl::error::Error;

#[test]
fn layout_modules_exist() {
    let _: Option<Config> = None;
    let _ = (
        scrawl::recognizer::MODULE,
        scrawl::device::list_mice_in as fn(&std::path::Path) -> _,
        scrawl::sys::MODULE,
    );
}

#[test]
fn error_displays_message() {
    let e = Error::Unsupported("grab");
    assert_eq!(e.to_string(), "unsupported: grab");
}
