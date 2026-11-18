# Changelog
All notable changes are documented here. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning: [Semantic Versioning](https://semver.org/).

## [Unreleased]
- `app = "<WM_CLASS or app_id>"` on `[[binding]]` and `[[profile.binding]]`, and `[[profile]] app = ...` (optionally with `device`): bindings limited to one app, matched whole and case-insensitively, falling back to the global ones; `check-config` validates them. `run` does not track the focused window yet, so only bindings without `app` fire for now.
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
