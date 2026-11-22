//! M4.3: hot reload of the config file.
#![allow(clippy::expect_used)] // test helpers outside #[test] fns

use std::path::Path;
use std::time::{Duration, Instant};

use scrawl::config::Config;
use scrawl::error::Error;
use scrawl::reload::ConfigWatcher;

const V1: &str =
    "trigger = \"right\"\n\n[[binding]]\nstroke = \"L\"\naction = { key = \"alt+left\" }\n";
const V2: &str =
    "trigger = \"right\"\n\n[[binding]]\nstroke = \"L\"\naction = { key = \"ctrl+w\" }\n";

/// Poll until the watcher reports a change or the deadline passes.
fn wait_for(w: &mut ConfigWatcher) -> Option<Result<Config, Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(r) = w.poll() {
            return Some(r);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

fn write(path: &Path, src: &str) {
    std::fs::write(path, src).expect("write config");
}

#[test]
fn unchanged_file_reports_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    write(&path, V1);
    let mut w = ConfigWatcher::new(&path).expect("watch");
    std::thread::sleep(Duration::from_millis(200));
    assert!(w.poll().is_none());
}

#[test]
fn edited_file_is_reloaded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    write(&path, V1);
    let mut w = ConfigWatcher::new(&path).expect("watch");
    write(&path, V2);
    let cfg = wait_for(&mut w).expect("a reload").expect("valid config");
    assert_eq!(
        cfg.binding[0].action,
        scrawl::config::Action::Key("ctrl+w".into())
    );
}

#[test]
fn file_replaced_by_rename_is_reloaded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    write(&path, V1);
    let mut w = ConfigWatcher::new(&path).expect("watch");
    let tmp = dir.path().join("config.toml.swp");
    write(&tmp, V2);
    std::fs::rename(&tmp, &path).expect("rename over config");
    let cfg = wait_for(&mut w).expect("a reload").expect("valid config");
    assert_eq!(
        cfg.binding[0].action,
        scrawl::config::Action::Key("ctrl+w".into())
    );
}

#[test]
fn invalid_edit_is_reported_and_next_good_edit_still_loads() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    write(&path, V1);
    let mut w = ConfigWatcher::new(&path).expect("watch");
    write(&path, "trigger = \"nope\"\n");
    let err = wait_for(&mut w).expect("a reload");
    assert!(matches!(err, Err(Error::Config { .. })), "got {err:?}");
    write(&path, V2);
    let cfg = wait_for(&mut w).expect("a reload").expect("valid config");
    assert_eq!(
        cfg.binding[0].action,
        scrawl::config::Action::Key("ctrl+w".into())
    );
}

#[test]
fn watching_a_missing_directory_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("no-such-dir").join("config.toml");
    assert!(ConfigWatcher::new(&path).is_err());
}
