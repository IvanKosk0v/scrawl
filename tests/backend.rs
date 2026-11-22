//! M5.5: the window backend comes from the session environment; Wayland wins over XWayland's DISPLAY.

use std::ffi::OsString;

use scrawl::backend::{display_server, window_backend, DisplayServer, WindowBackend};

/// A fake environment holding exactly `vars`.
fn env(vars: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
    move |k| {
        vars.iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| OsString::from(v))
    }
}

fn pick(vars: &'static [(&'static str, &'static str)]) -> (DisplayServer, WindowBackend) {
    (display_server(env(vars)), window_backend(env(vars)))
}

#[test]
fn nothing_set_is_none_and_unsupported() {
    assert_eq!(pick(&[]), (DisplayServer::None, WindowBackend::Unsupported));
}

#[test]
fn display_only_is_x11() {
    assert_eq!(
        pick(&[("DISPLAY", ":0")]),
        (DisplayServer::X11, WindowBackend::X11)
    );
}

#[test]
fn wayland_only_is_wayland() {
    assert_eq!(
        pick(&[("WAYLAND_DISPLAY", "wayland-0")]),
        (DisplayServer::Wayland, WindowBackend::Unsupported)
    );
}

#[test]
fn xwayland_both_set_picks_wayland() {
    // GNOME / KDE: both displays, no compositor IPC variable => Wayland, global bindings only.
    assert_eq!(
        pick(&[("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "wayland-0")]),
        (DisplayServer::Wayland, WindowBackend::Unsupported)
    );
}

#[test]
fn sway_and_hyprland_detected_under_xwayland() {
    assert_eq!(
        pick(&[
            ("DISPLAY", ":0"),
            ("WAYLAND_DISPLAY", "wayland-1"),
            ("SWAYSOCK", "/run/user/1000/sway-ipc.sock")
        ]),
        (DisplayServer::Wayland, WindowBackend::Sway)
    );
    assert_eq!(
        pick(&[
            ("DISPLAY", ":1"),
            ("WAYLAND_DISPLAY", "wayland-1"),
            ("HYPRLAND_INSTANCE_SIGNATURE", "abc_123")
        ]),
        (DisplayServer::Wayland, WindowBackend::Hyprland)
    );
}

#[test]
fn hyprland_wins_when_both_compositor_vars_are_set() {
    assert_eq!(
        window_backend(env(&[
            ("WAYLAND_DISPLAY", "wayland-1"),
            ("SWAYSOCK", "/tmp/s"),
            ("HYPRLAND_INSTANCE_SIGNATURE", "abc")
        ])),
        WindowBackend::Hyprland
    );
}

#[test]
fn empty_values_count_as_unset() {
    assert_eq!(
        pick(&[("WAYLAND_DISPLAY", ""), ("DISPLAY", ":0")]),
        (DisplayServer::X11, WindowBackend::X11)
    );
    assert_eq!(
        pick(&[("WAYLAND_DISPLAY", "wayland-0"), ("SWAYSOCK", "")]),
        (DisplayServer::Wayland, WindowBackend::Unsupported)
    );
    assert_eq!(
        pick(&[("DISPLAY", "")]),
        (DisplayServer::None, WindowBackend::Unsupported)
    );
}

#[test]
fn compositor_vars_ignored_without_wayland() {
    assert_eq!(
        pick(&[("DISPLAY", ":0"), ("SWAYSOCK", "/tmp/stale.sock")]),
        (DisplayServer::X11, WindowBackend::X11)
    );
    assert_eq!(
        pick(&[("HYPRLAND_INSTANCE_SIGNATURE", "abc")]),
        (DisplayServer::None, WindowBackend::Unsupported)
    );
}
