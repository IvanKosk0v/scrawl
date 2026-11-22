//! M2.1: mouse enumeration (read-only, no grab).

use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;

use evdev::{AttributeSet, KeyCode, RelativeAxisCode};
use scrawl::device::{is_mouse, list_mice_in};
use scrawl::error::Error;

fn keys(codes: &[KeyCode]) -> AttributeSet<KeyCode> {
    codes.iter().copied().collect()
}

fn rels(codes: &[RelativeAxisCode]) -> AttributeSet<RelativeAxisCode> {
    codes.iter().copied().collect()
}

#[test]
fn mouse_has_left_button_and_both_relative_axes() {
    let k = keys(&[KeyCode::BTN_LEFT, KeyCode::BTN_RIGHT]);
    let r = rels(&[RelativeAxisCode::REL_X, RelativeAxisCode::REL_Y]);
    assert!(is_mouse(Some(&k), Some(&r)));
}

#[test]
fn non_mice_are_rejected() {
    let k = keys(&[KeyCode::BTN_LEFT]);
    let only_x = rels(&[RelativeAxisCode::REL_X]);
    let both = rels(&[RelativeAxisCode::REL_X, RelativeAxisCode::REL_Y]);
    let keyboard = keys(&[KeyCode::KEY_A]);
    assert!(!is_mouse(Some(&k), Some(&only_x)));
    assert!(!is_mouse(Some(&keyboard), Some(&both)));
    assert!(!is_mouse(None, Some(&both)));
    assert!(!is_mouse(Some(&k), None));
}

#[test]
fn unreadable_device_gives_helpful_permission_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let node = dir.path().join("event0");
    File::create(&node).expect("create");
    fs::set_permissions(&node, fs::Permissions::from_mode(0o000)).expect("chmod");
    if File::open(&node).is_ok() {
        return; // running as root: permissions are not enforced
    }
    match list_mice_in(dir.path()) {
        Err(e @ Error::PermissionDenied(_)) => {
            let msg = e.to_string();
            assert!(msg.contains("event0"), "{msg}");
            assert!(msg.contains("`input` group"), "{msg}");
            assert!(msg.contains("udev"), "{msg}");
        }
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
}

#[test]
fn non_device_files_are_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    File::create(dir.path().join("event1")).expect("create"); // not an evdev node
    File::create(dir.path().join("mouse0")).expect("create");
    fs::create_dir(dir.path().join("by-id")).expect("mkdir");
    assert!(list_mice_in(dir.path()).expect("list").is_empty());
}

#[test]
fn missing_directory_is_an_io_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nope");
    assert!(matches!(list_mice_in(&missing), Err(Error::Io { .. })));
}
