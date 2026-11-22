//! M3.1: uinput virtual mouse mirroring the source capabilities (no grab).

mod common;

use std::path::PathBuf;

use common::{fake_caps, open_when_ready};
use evdev::{EventType, KeyCode, RelativeAxisCode};
use scrawl::config::Escape;
use scrawl::device::virtual_mouse::{is_virtual_name, virtual_name, VirtualMouse, MAX_NAME_LEN};
use scrawl::error::Error;

#[test]
fn virtual_name_has_prefix() {
    let n = virtual_name("Logitech USB Optical Mouse");
    assert_eq!(n, "scrawl: Logitech USB Optical Mouse");
    assert!(is_virtual_name(&n));
}

#[test]
fn virtual_name_truncates_to_uinput_limit() {
    let long = "ü".repeat(100);
    let n = virtual_name(&long);
    assert!(n.len() <= MAX_NAME_LEN, "{} bytes", n.len());
    const { assert!(MAX_NAME_LEN + 1 < 80, "evdev asserts name.len() + 1 < 80") };
    assert!(n.starts_with("scrawl: ü"));
    assert!(n.ends_with('ü'), "cut on a char boundary");
}

#[test]
fn is_virtual_name_detects_own_devices() {
    assert!(is_virtual_name("scrawl: x"));
    assert!(!is_virtual_name("Logitech USB Optical Mouse"));
    assert!(!is_virtual_name("my scrawl: mouse"));
}

#[test]
fn uinput_denied_message_has_hint() {
    let msg = Error::UinputDenied(PathBuf::from("/dev/uinput")).to_string();
    assert!(msg.contains("/dev/uinput"), "{msg}");
    assert!(msg.contains("input"), "{msg}");
    assert!(msg.contains("udev"), "{msg}");
}

/// Needs a writable /dev/uinput; run with `cargo test --test virtual_mouse -- --ignored`.
#[test]
#[ignore = "needs writable /dev/uinput"]
fn uinput_mirrors_fake_caps() {
    let caps = fake_caps("Fake Test Mouse");
    let mut vm = VirtualMouse::create(&caps).expect("create virtual mouse");
    let node = vm.dev_node().expect("dev node");
    let dev = open_when_ready(&node);

    assert_eq!(dev.name(), Some("scrawl: Fake Test Mouse"));
    assert_eq!(dev.input_id(), caps.id);
    let keys: Vec<_> = dev
        .supported_keys()
        .into_iter()
        .flat_map(|k| k.iter())
        .collect();
    assert_eq!(keys, caps.keys.iter().collect::<Vec<_>>());
    let rel: Vec<_> = dev
        .supported_relative_axes()
        .into_iter()
        .flat_map(|r| r.iter())
        .collect();
    assert_eq!(rel, caps.rel.iter().collect::<Vec<_>>());
    let msc: Vec<_> = dev
        .misc_properties()
        .into_iter()
        .flat_map(|m| m.iter())
        .collect();
    assert_eq!(msc, caps.msc.iter().collect::<Vec<_>>());

    let mut dev = dev;
    vm.emit(&[evdev::InputEvent::new(
        EventType::RELATIVE.0,
        RelativeAxisCode::REL_X.0,
        7,
    )])
    .expect("emit");
    let got: Vec<_> = dev.fetch_events().expect("fetch").collect();
    assert!(
        got.iter().any(|e| e.event_type() == EventType::RELATIVE
            && e.code() == RelativeAxisCode::REL_X.0
            && e.value() == 7),
        "{got:?}"
    );
}

#[test]
fn escape_check_accepts_chord_the_mouse_has() {
    let caps = fake_caps("Has All Buttons");
    assert!(caps.check_escape(&Escape::default()).is_ok());
}

#[test]
fn escape_check_rejects_missing_button() {
    let mut caps = fake_caps("No Middle Button");
    caps.keys.remove(KeyCode::BTN_MIDDLE);
    let err = caps
        .check_escape(&Escape::default())
        .expect_err("middle is missing");
    let msg = err.to_string();
    assert!(matches!(err, Error::EscapeUnavailable { .. }), "{err:?}");
    assert!(msg.contains("middle"), "{msg}");
    assert!(msg.contains("left+right+middle"), "{msg}");
    assert!(msg.contains("escape"), "{msg}");
}
