//! M4.1: key-combo parsing and the event sequence sent through the uinput virtual keyboard.

mod common;

use evdev::{EventType, KeyCode};
use proptest::prelude::*;
use strokebind::actions::key::{keyboard_keys, KeyCombo};
use strokebind::device::virtual_keyboard::VirtualKeyboard;

fn combo(s: &str) -> KeyCombo {
    match s.parse() {
        Ok(c) => c,
        Err(e) => panic!("`{s}` should parse: {e}"),
    }
}

fn parse_err(s: &str) -> String {
    match s.parse::<KeyCombo>() {
        Ok(c) => panic!("`{s}` should fail, got {c:?}"),
        Err(e) => e,
    }
}

/// (code, value) of each report; every report must hold exactly one key event.
fn sequence(c: &KeyCombo) -> Vec<(KeyCode, i32)> {
    c.reports()
        .into_iter()
        .map(|r| {
            assert_eq!(r.len(), 1, "one key event per report: {r:?}");
            assert_eq!(r[0].event_type(), EventType::KEY);
            (KeyCode::new(r[0].code()), r[0].value())
        })
        .collect()
}

#[test]
fn parses_modifiers_and_key() {
    let c = combo("ctrl+shift+t");
    assert_eq!(
        c.modifiers(),
        &[KeyCode::KEY_LEFTCTRL, KeyCode::KEY_LEFTSHIFT]
    );
    assert_eq!(c.key(), KeyCode::KEY_T);
}

#[test]
fn names_are_case_insensitive_and_aliases_work() {
    let cases = [
        ("Alt+Left", vec![KeyCode::KEY_LEFTALT], KeyCode::KEY_LEFT),
        ("super+1", vec![KeyCode::KEY_LEFTMETA], KeyCode::KEY_1),
        (
            "control+escape",
            vec![KeyCode::KEY_LEFTCTRL],
            KeyCode::KEY_ESC,
        ),
        (
            "meta+return",
            vec![KeyCode::KEY_LEFTMETA],
            KeyCode::KEY_ENTER,
        ),
        ("altgr+e", vec![KeyCode::KEY_RIGHTALT], KeyCode::KEY_E),
        (" ctrl + w ", vec![KeyCode::KEY_LEFTCTRL], KeyCode::KEY_W),
        ("f5", vec![], KeyCode::KEY_F5),
        ("volumeup", vec![], KeyCode::KEY_VOLUMEUP),
        ("pgdn", vec![], KeyCode::KEY_PAGEDOWN),
    ];
    for (s, mods, key) in cases {
        let c = combo(s);
        assert_eq!(c.modifiers(), mods.as_slice(), "{s}");
        assert_eq!(c.key(), key, "{s}");
    }
}

#[test]
fn modifier_alone_is_the_key() {
    let c = combo("super");
    assert!(c.modifiers().is_empty());
    assert_eq!(c.key(), KeyCode::KEY_LEFTMETA);
}

#[test]
fn invalid_combos_are_rejected_with_a_reason() {
    assert!(parse_err("").contains("empty"));
    assert!(parse_err("ctrl+").contains("empty"));
    assert!(parse_err("ctrl++t").contains("empty"));
    assert!(parse_err("t+ctrl").contains("modifier"));
    assert!(parse_err("ctrl+ctrl+t").contains("twice"));
    assert!(parse_err("ctrl+banana").contains("banana"));
    // Mouse buttons are not keyboard keys.
    assert!(parse_err("btn_left").contains("btn_left"));
}

#[test]
fn emitted_sequence_presses_in_order_and_releases_in_reverse() {
    assert_eq!(
        sequence(&combo("ctrl+shift+t")),
        vec![
            (KeyCode::KEY_LEFTCTRL, 1),
            (KeyCode::KEY_LEFTSHIFT, 1),
            (KeyCode::KEY_T, 1),
            (KeyCode::KEY_T, 0),
            (KeyCode::KEY_LEFTSHIFT, 0),
            (KeyCode::KEY_LEFTCTRL, 0),
        ]
    );
}

#[test]
fn emitted_sequence_for_a_single_key() {
    assert_eq!(
        sequence(&combo("f5")),
        vec![(KeyCode::KEY_F5, 1), (KeyCode::KEY_F5, 0)]
    );
}

#[test]
fn virtual_keyboard_declares_every_parsable_key() {
    let keys = keyboard_keys();
    for s in [
        "ctrl+shift+t",
        "super+1",
        "altgr+e",
        "volumeup",
        "f24",
        "playpause",
    ] {
        let c = combo(s);
        for k in c.modifiers().iter().chain(std::iter::once(&c.key())) {
            assert!(keys.contains(*k), "{s}: {k:?} not declared");
        }
    }
    assert!(
        !keys.contains(KeyCode::BTN_LEFT),
        "no mouse buttons on the keyboard"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn parsing_never_panics_and_parsed_keys_are_declared(s in "[a-zA-Z0-9_+ ]{0,24}") {
        if let Ok(c) = s.parse::<KeyCombo>() {
            let keys = keyboard_keys();
            prop_assert!(keys.contains(c.key()));
            prop_assert!(c.modifiers().iter().all(|m| keys.contains(*m)));
        }
    }
}

/// Needs a writable /dev/uinput. Types shift+F24 into the session, which nothing binds by default.
#[test]
#[ignore = "needs writable /dev/uinput"]
fn virtual_keyboard_emits_the_sequence() {
    let c = combo("shift+f24");
    let mut kb = match VirtualKeyboard::create() {
        Ok(kb) => kb,
        Err(e) => panic!("create keyboard: {e}"),
    };
    let node = match kb.dev_node() {
        Ok(n) => n,
        Err(e) => panic!("keyboard node: {e}"),
    };
    let mut reader = common::open_when_ready(&node);
    assert_eq!(reader.name(), Some("strokebind: keyboard"));
    if let Err(e) = kb.press(&c) {
        panic!("press: {e}");
    }
    let mut got = Vec::new();
    while got.len() < 4 {
        let events = match reader.fetch_events() {
            Ok(ev) => ev,
            Err(e) => panic!("read: {e}"),
        };
        got.extend(
            events
                .filter(|e| e.event_type() == EventType::KEY)
                .map(|e| (KeyCode::new(e.code()), e.value())),
        );
    }
    assert_eq!(got, sequence(&c));
}
