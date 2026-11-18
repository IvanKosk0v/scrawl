//! Key combos like `ctrl+shift+t`: parsing and the key event sequence that performs them. Pure logic.

use std::str::FromStr;

use evdev::{AttributeSet, EventType, InputEvent, KeyCode};

/// Friendly names that differ from the evdev `KEY_*` name (which is accepted too, without the prefix).
const ALIASES: &[(&str, KeyCode)] = &[
    ("ctrl", KeyCode::KEY_LEFTCTRL),
    ("control", KeyCode::KEY_LEFTCTRL),
    ("shift", KeyCode::KEY_LEFTSHIFT),
    ("alt", KeyCode::KEY_LEFTALT),
    ("altgr", KeyCode::KEY_RIGHTALT),
    ("super", KeyCode::KEY_LEFTMETA),
    ("meta", KeyCode::KEY_LEFTMETA),
    ("win", KeyCode::KEY_LEFTMETA),
    ("logo", KeyCode::KEY_LEFTMETA),
    ("return", KeyCode::KEY_ENTER),
    ("escape", KeyCode::KEY_ESC),
    ("del", KeyCode::KEY_DELETE),
    ("ins", KeyCode::KEY_INSERT),
    ("pgup", KeyCode::KEY_PAGEUP),
    ("pgdn", KeyCode::KEY_PAGEDOWN),
    ("period", KeyCode::KEY_DOT),
    ("print", KeyCode::KEY_SYSRQ),
];

const MODIFIERS: &[KeyCode] = &[
    KeyCode::KEY_LEFTCTRL,
    KeyCode::KEY_RIGHTCTRL,
    KeyCode::KEY_LEFTSHIFT,
    KeyCode::KEY_RIGHTSHIFT,
    KeyCode::KEY_LEFTALT,
    KeyCode::KEY_RIGHTALT,
    KeyCode::KEY_LEFTMETA,
    KeyCode::KEY_RIGHTMETA,
];

/// Keyboard key codes (no mouse/joystick `BTN_*` ranges): `1..=0xff` and `0x160..=0x2bf`.
fn is_keyboard_key(code: KeyCode) -> bool {
    matches!(code.code(), 1..=0xff | 0x160..=0x2bf)
}

/// Every key the virtual keyboard declares; any parsed [`KeyCombo`] only uses these.
pub fn keyboard_keys() -> AttributeSet<KeyCode> {
    (1..=0x2bf_u16)
        .map(KeyCode::new)
        .filter(|&k| is_keyboard_key(k))
        .collect()
}

fn key_by_name(name: &str) -> Option<KeyCode> {
    let lower = name.to_ascii_lowercase();
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == lower)
        .map(|&(_, code)| code)
        .or_else(|| KeyCode::from_str(&format!("KEY_{}", lower.to_ascii_uppercase())).ok())
        .filter(|&k| is_keyboard_key(k))
}

/// A validated combo: zero or more modifiers followed by one key, e.g. `alt+left`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombo {
    modifiers: Vec<KeyCode>,
    key: KeyCode,
}

impl KeyCombo {
    /// Modifiers in the order written.
    pub fn modifiers(&self) -> &[KeyCode] {
        &self.modifiers
    }

    /// The final key (may itself be a modifier, e.g. `super`).
    pub fn key(&self) -> KeyCode {
        self.key
    }

    /// Reports to emit, one key event each: modifiers down in order, key down, key up, modifiers up in reverse.
    pub fn reports(&self) -> Vec<Vec<InputEvent>> {
        let ev = |k: KeyCode, v| vec![InputEvent::new(EventType::KEY.0, k.code(), v)];
        let mut out: Vec<_> = self.modifiers.iter().map(|&m| ev(m, 1)).collect();
        out.push(ev(self.key, 1));
        out.push(ev(self.key, 0));
        out.extend(self.modifiers.iter().rev().map(|&m| ev(m, 0)));
        out
    }
}

impl FromStr for KeyCombo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut codes = Vec::new();
        for token in s.split('+').map(str::trim) {
            if token.is_empty() {
                return Err(format!("key combo `{s}` has an empty key name"));
            }
            let code = key_by_name(token).ok_or_else(|| {
                format!("unknown key `{token}` in `{s}` (e.g. ctrl, shift, alt, super, a, 1, f5, left, enter)")
            })?;
            if codes.contains(&code) {
                return Err(format!("key `{token}` appears twice in `{s}`"));
            }
            codes.push(code);
        }
        let Some(key) = codes.pop() else {
            return Err(format!("key combo `{s}` is empty"));
        };
        if let Some(&bad) = codes.iter().find(|c| !MODIFIERS.contains(c)) {
            return Err(format!(
                "`{s}`: only the last key may be a non-modifier, but {bad:?} comes earlier"
            ));
        }
        Ok(KeyCombo {
            modifiers: codes,
            key,
        })
    }
}
