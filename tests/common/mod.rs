//! Helpers shared by the uinput-backed integration tests.
#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use evdev::{AttributeSet, BusType, InputId, KeyCode, MiscCode, PropType, RelativeAxisCode};
use scrawl::device::virtual_mouse::MouseCaps;

/// Fake source: capabilities of a typical 5-button wheel mouse.
pub fn fake_caps(name: &str) -> MouseCaps {
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

/// udev applies the node's group a moment after creation, so retry for up to 2 s.
pub fn open_when_ready(node: &Path) -> evdev::Device {
    let mut last = None;
    for _ in 0..40 {
        match scrawl::device::source::open_readonly(node) {
            Ok(dev) => return dev,
            Err(e) => last = Some(e),
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("open {}: {last:?}", node.display());
}
