//! X11 active-window adapter: EWMH `_NET_ACTIVE_WINDOW` plus `WM_CLASS`.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

use crate::window::{AppId, WindowAdapter};

/// The window id held by a `_NET_ACTIVE_WINDOW` property value; `0` means no active window.
pub fn parse_active_window(value: &[u32]) -> Option<u32> {
    value.first().copied().filter(|&id| id != 0)
}

/// The app id held by a `WM_CLASS` property value (`instance\0class\0`): the class, else the
/// instance when the class is blank.
pub fn parse_wm_class(value: &[u8]) -> Option<AppId> {
    let mut parts = value.split(|&b| b == 0);
    let instance = parts
        .next()
        .and_then(|p| AppId::new(std::str::from_utf8(p).ok()?));
    let class = parts
        .next()
        .and_then(|p| AppId::new(std::str::from_utf8(p).ok()?));
    class.or(instance)
}

type LookupResult = Result<Option<AppId>, Box<dyn std::error::Error>>;

fn lookup() -> LookupResult {
    let (conn, screen) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen].root;
    let atom = conn.intern_atom(true, b"_NET_ACTIVE_WINDOW")?.reply()?.atom;
    let prop = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)?
        .reply()?;
    let ids: Vec<u32> = prop.value32().map(Iterator::collect).unwrap_or_default();
    let Some(window) = parse_active_window(&ids) else {
        return Ok(None);
    };
    let class = conn
        .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)?
        .reply()?;
    Ok(parse_wm_class(&class.value))
}

/// Reads the focused window's `WM_CLASS` from the X server on every call.
#[derive(Debug, Clone, Copy, Default)]
pub struct X11Adapter;

impl WindowAdapter for X11Adapter {
    fn active_window(&self) -> Option<AppId> {
        lookup().unwrap_or_else(|e| {
            eprintln!("x11 window lookup failed: {e}");
            None
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> Option<AppId> {
        AppId::new(s)
    }

    #[test]
    fn active_window_takes_first_nonzero_id() {
        assert_eq!(parse_active_window(&[0x40_0007]), Some(0x40_0007));
        assert_eq!(parse_active_window(&[0x40_0007, 9]), Some(0x40_0007));
    }

    #[test]
    fn active_window_none_or_empty_is_unknown() {
        assert_eq!(parse_active_window(&[0]), None);
        assert_eq!(parse_active_window(&[]), None);
    }

    #[test]
    fn wm_class_uses_the_class_part() {
        assert_eq!(parse_wm_class(b"Navigator\0firefox\0"), id("firefox"));
        assert_eq!(parse_wm_class(b"kitty\0kitty"), id("kitty"));
    }

    #[test]
    fn wm_class_falls_back_to_instance() {
        assert_eq!(parse_wm_class(b"xterm\0\0"), id("xterm"));
        assert_eq!(parse_wm_class(b"xterm"), id("xterm"));
    }

    #[test]
    fn wm_class_blank_or_garbage_is_unknown() {
        assert_eq!(parse_wm_class(b""), None);
        assert_eq!(parse_wm_class(b"\0\0"), None);
        assert_eq!(parse_wm_class(b"  \0 \0"), None);
        assert_eq!(parse_wm_class(&[0xff, 0xfe, 0]), None);
    }
}
