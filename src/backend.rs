//! Which display server and window backend this session runs, decided from its environment.
//!
//! Pure: callers pass an environment lookup (`std::env::var_os` in `main`). XWayland sets both
//! `WAYLAND_DISPLAY` and `DISPLAY`; Wayland wins, since an X11 query would only see XWayland
//! windows. Empty values count as unset.

use std::ffi::OsString;

/// The display server the session uses (also picks the M7 overlay backend).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    /// `WAYLAND_DISPLAY` is set (with or without XWayland's `DISPLAY`).
    Wayland,
    /// Only `DISPLAY` is set.
    X11,
    /// Neither (e.g. a text console or an ssh session).
    None,
}

/// Where per-app profiles read the focused window from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowBackend {
    /// EWMH `_NET_ACTIVE_WINDOW` on an X11 session.
    X11,
    /// Sway IPC (`SWAYSOCK` under Wayland).
    Sway,
    /// Hyprland IPC (`HYPRLAND_INSTANCE_SIGNATURE` under Wayland).
    Hyprland,
    /// No supported method (KDE, GNOME, other compositors, no display): global bindings only.
    Unsupported,
}

/// True when `key` is set to a non-empty value.
fn is_set(env: &impl Fn(&str) -> Option<OsString>, key: &str) -> bool {
    env(key).is_some_and(|v| !v.is_empty())
}

/// The session's display server: `WAYLAND_DISPLAY` first, then `DISPLAY`.
pub fn display_server(env: impl Fn(&str) -> Option<OsString>) -> DisplayServer {
    if is_set(&env, "WAYLAND_DISPLAY") {
        DisplayServer::Wayland
    } else if is_set(&env, "DISPLAY") {
        DisplayServer::X11
    } else {
        DisplayServer::None
    }
}

/// The window backend: on Wayland, Hyprland then Sway by their IPC variables (trusted only under
/// Wayland, so a stale `SWAYSOCK` in an X11 session is ignored); on X11, EWMH.
pub fn window_backend(env: impl Fn(&str) -> Option<OsString>) -> WindowBackend {
    match display_server(&env) {
        DisplayServer::Wayland if is_set(&env, "HYPRLAND_INSTANCE_SIGNATURE") => {
            WindowBackend::Hyprland
        }
        DisplayServer::Wayland if is_set(&env, "SWAYSOCK") => WindowBackend::Sway,
        DisplayServer::Wayland | DisplayServer::None => WindowBackend::Unsupported,
        DisplayServer::X11 => WindowBackend::X11,
    }
}
