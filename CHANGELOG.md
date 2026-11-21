# Changelog
All notable changes are documented here. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning: [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-11-21
- README rewritten for launch: pitch, install, quick start, config examples (checked by tests), comparison table, FAQ and a demo GIF (`docs/demo.gif`).
- `.deb` package via cargo-deb (`cargo deb`): installs `/usr/bin/strokebind`, the systemd user unit, the udev rule (reloaded in `postinst`) and an example config; a `v*` tag runs `.github/workflows/release.yml`, which builds, installs and version-checks the package and attaches it to a GitHub release.
- `packaging/install.sh` installs the binary, a systemd user unit (`strokebind.service`) and a starter config under your home directory; `packaging/70-strokebind.rules` (udev, `uaccess`) gives the seat user access to `/dev/uinput` and mice, so no daemon runs as root. The script prints the one root step instead of running it.
- README support matrix: per-app bindings on X11, Sway and Hyprland; KDE Plasma and GNOME use global bindings only.
- On Hyprland, `run` reads the focused window's class over the Hyprland IPC socket at each trigger press, so `app` bindings fire there too.
- On Sway, `run` reads the focused window (`app_id`, or the XWayland class) over the `SWAYSOCK` IPC at each trigger press, so `app` bindings fire there too.
- On X11, `run` reads the focused window (EWMH `_NET_ACTIVE_WINDOW` + `WM_CLASS` class) at each trigger press, so `app` bindings now fire there; other desktops still use only global bindings.
- `app = "<WM_CLASS or app_id>"` on `[[binding]]` and `[[profile.binding]]`, and `[[profile]] app = ...` (optionally with `device`): bindings limited to one app, matched whole and case-insensitively, falling back to the global ones; `check-config` validates them.
- `run` creates the virtual keyboard at startup (and after a reload) when a binding uses `key`, so the first key action is not lost; a profile with an empty `device` is rejected.
- `run` now moves the pointer live while the trigger is held; a release within `click_slop_px` (default 6, must be less than every `min_stroke_px` unless `freeze_pointer = true`) of the press is a click, a recognized stroke runs its action, anything else is swallowed. `freeze_pointer = true` restores the previous frozen-pointer behaviour.
- `feedback = "notify"` (default `"off"`): `run` shows each recognized stroke, or `<stroke>: no match`, as a desktop notification via `notify-send`.
- `strokebind check-config FILE` validates a config: prints `OK` with binding/profile counts, or the error with its line number and the offending line quoted (exit 1).
- `run` performs the bound action (key combo or command) for each recognized stroke; invalid key combos and empty `run` commands are rejected when the config loads.
- `[[profile]]` sections (`device = "<name substring>"`, optional `trigger`, `min_stroke_px`, `escape`, `[[profile.binding]]`) override settings per mouse.
- `run --config FILE` reloads bindings when the file changes (a broken edit is reported and ignored); trigger, `min_stroke_px` and `escape` need a restart.
- `strokebind run [--device PATH] [--config FILE] [--timeout N]` grabs the mouse, forwards it through a virtual mouse (plain clicks replayed unchanged) and prints recognized strokes.
- `run` refuses to start when the escape chord uses a button the mouse does not have.
- Failsafes for `run`: escape chord (config `escape`, default `left+right+middle`), `--timeout N`, and release of the grab on Ctrl-C, SIGTERM or a crash.
- `list-devices` and `record` skip strokebind's own virtual mice (names starting with `strokebind: `).
- `strokebind record [--device PATH] [--config FILE]` prints the recognized stroke after each trigger release (read-only, no grab).
- `strokebind list-devices` lists mice (name and path) and explains how to fix permission errors.
- Config file parsing (TOML) with line-numbered validation errors.
- CLI skeleton: `record`, `run`, `check-config`, `list-devices` subcommands (stubs).
