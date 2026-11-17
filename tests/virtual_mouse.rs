//! M3.1: uinput virtual mouse mirroring the source capabilities (no grab).

use std::path::PathBuf;

use evdev::{
    AttributeSet, BusType, EventType, InputId, KeyCode, MiscCode, PropType, RelativeAxisCode,
};
use strokebind::device::virtual_mouse::{
    is_virtual_name, virtual_name, MouseCaps, VirtualMouse, MAX_NAME_LEN,
};
use strokebind::error::Error;

/// Fake source: capabilities of a typical 5-button wheel mouse.
fn fake_caps(name: &str) -> MouseCaps {
    MouseCaps {
        name: name.to_owned(),
        id: InputId::new(BusType::BUS_USB, 0x046d, 0xc077, 0x0111),
        keys: [
            KeyCode::BTN_LEFT,
            KeyCode::BTN_RIGHT,
            KeyCode::BTN_MIDDLE,
            KeyCode::BTN_SIDE,
            KeyCode::BTN_EXTRA,
        ]
        .into_iter()
        .collect(),
        rel: [
            RelativeAxisCode::REL_X,
            RelativeAxisCode::REL_Y,
            RelativeAxisCode::REL_WHEEL,
            RelativeAxisCode::REL_WHEEL_HI_RES,
        ]
        .into_iter()
        .collect(),
        msc: [MiscCode::MSC_SCAN].into_iter().collect(),
        props: AttributeSet::<PropType>::new(),
    }
}

#[test]
fn virtual_name_has_prefix() {
    let n = virtual_name("Logitech USB Optical Mouse");
    assert_eq!(n, "strokebind: Logitech USB Optical Mouse");
    assert!(is_virtual_name(&n));
}

#[test]
fn virtual_name_truncates_to_uinput_limit() {
    let long = "ü".repeat(100);
    let n = virtual_name(&long);
    assert!(n.len() <= MAX_NAME_LEN, "{} bytes", n.len());
    const { assert!(MAX_NAME_LEN + 1 < 80, "evdev asserts name.len() + 1 < 80") };
    assert!(n.starts_with("strokebind: ü"));
    assert!(n.ends_with('ü'), "cut on a char boundary");
}

#[test]
fn is_virtual_name_detects_own_devices() {
    assert!(is_virtual_name("strokebind: x"));
    assert!(!is_virtual_name("Logitech USB Optical Mouse"));
    assert!(!is_virtual_name("my strokebind: mouse"));
}

#[test]
fn uinput_denied_message_has_hint() {
    let msg = Error::UinputDenied(PathBuf::from("/dev/uinput")).to_string();
    assert!(msg.contains("/dev/uinput"), "{msg}");
    assert!(msg.contains("input"), "{msg}");
    assert!(msg.contains("udev"), "{msg}");
}

/// udev applies the node's group a moment after creation, so retry for up to 2 s.
fn open_when_ready(node: &std::path::Path) -> evdev::Device {
    let mut last = None;
    for _ in 0..40 {
        match strokebind::device::source::open_readonly(node) {
            Ok(dev) => return dev,
            Err(e) => last = Some(e),
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("open {}: {last:?}", node.display());
}

/// Needs a writable /dev/uinput; run with `cargo test --test virtual_mouse -- --ignored`.
#[test]
#[ignore = "needs writable /dev/uinput"]
fn uinput_mirrors_fake_caps() {
    let caps = fake_caps("Fake Test Mouse");
    let mut vm = VirtualMouse::create(&caps).expect("create virtual mouse");
    let node = vm.dev_node().expect("dev node");
    let dev = open_when_ready(&node);

    assert_eq!(dev.name(), Some("strokebind: Fake Test Mouse"));
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
