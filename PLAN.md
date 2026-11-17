# PLAN (rigid). Changes only via `/approve P<n>`.

Legend: [ ] todo, [x] done. Every task ends with `./scripts/check.sh` OK and follows CLAUDE.md "Definition of done".
Branch per milestone: m0-foundation, m1-recognizer, m2-devices, m3-grab, m4-actions, m5-adapters, m6-launch.

## M0 Foundation
- [x] M0.1 Crate layout: `src/{main,lib,error,config}.rs`, `src/recognizer/`, `src/device/`, `src/sys/`. Acceptance: builds, check.sh OK.
- [x] M0.2 CLI skeleton with clap: `record`, `run`, `check-config`, `list-devices` (stubs). Acceptance: `strokebind --help` lists them; test asserts it.
- [x] M0.3 Config types + TOML parse + validation errors with line info. Acceptance: tests for valid, missing field, unknown stroke string.
- [x] M0.4 Repo gates: determine real MSRV with cargo-msrv and set `rust-version`; make `cargo deny check` pass with the deps added so far; make check.sh run it. Acceptance: both succeed locally; CI file unchanged unless needed.

## M1 Recognizer (pure logic, no I/O)
- [x] M1.1 Point stream -> quantized 8-direction segments (min length, jitter filter). Acceptance: tests for straight lines in 8 directions.
- [x] M1.2 Segments -> stroke string like `R`, `DR`, `L-U` (merge repeats, drop tiny segments). Acceptance: tests for L-shape, zigzag noise, tiny wiggle = no stroke.
- [x] M1.3 Matcher: stroke string -> binding, configurable tolerance. Acceptance: tests incl. ambiguous cases.
- [x] M1.4 Property tests (proptest): never panics on arbitrary points; same stroke under uniform scaling and small jitter; empty/one-point input = no stroke. Acceptance: proptests pass with >= 256 cases.

## M2 Device layer (read-only, NO grab)
- [x] M2.1 `list-devices`: enumerate mice via evdev. Acceptance: prints name + path; permission error gives a helpful message (test the message).
- [x] M2.2 Event loop: trigger button + REL_X/REL_Y -> points -> recognizer. Acceptance: integration test with a fake event source.
- [ ] M2.3 `record` prints the stroke after each trigger release. Acceptance: fake-source test; user verifies on real hardware.

## M3 Grab + passthrough (highest risk; use plan mode; strongest model advised)
- [ ] M3.1 uinput virtual mouse mirroring the source capabilities. Acceptance: tests on fake source; manual check.
- [ ] M3.2 Exclusive grab with guaranteed release: Drop, panic hook, SIGINT/SIGTERM. Acceptance: one test per release path.
- [ ] M3.3 Click passthrough: press held until stroke-vs-click is decided; plain click replayed unchanged. Acceptance: tests for click, stroke, press-move-return-click.
- [ ] M3.4 Failsafe: `--timeout N` auto-exit and a configurable escape key combo. Acceptance: documented and tested.

## M4 Actions + config
- [ ] M4.1 Action: key combo through uinput virtual keyboard. Acceptance: key-name parsing and emitted event sequence tests.
- [ ] M4.2 Action: run command (no shell by default, sanitized env). Acceptance: tests.
- [ ] M4.3 Bindings from config, per-device profiles, hot reload. Acceptance: reload test.
- [ ] M4.4 `check-config` with clear errors. Acceptance: golden-output tests.

## M5 Per-app profiles (adapters; global fallback always works)
- [ ] M5.1 Adapter trait `active_window() -> Option<AppId>`. Acceptance: mock-based tests.
- [ ] M5.2 Sway + Hyprland adapters (IPC sockets). Acceptance: parser tests with recorded JSON.
- [ ] M5.3 KDE + GNOME best-effort, or documented "global only". Acceptance: documented in README support matrix.

## M6 Packaging + launch
- [ ] M6.1 systemd user unit + udev rule + `install.sh` (no root-run daemon). Acceptance: fresh Ubuntu VM test notes.
- [ ] M6.2 `.deb` via cargo-deb + release workflow. Acceptance: installs; `strokebind --version` works.
- [ ] M6.3 README: pitch, 20s demo GIF, install, config example, support matrix, comparison table (docs/RESEARCH.md), FAQ. Acceptance: every claim maps to a tested feature; user approves the final text.
- [ ] M6.4 Release hygiene: CHANGELOG 0.1.0 section, tag v0.1.0, verify LICENSE headers, SECURITY contact works. Acceptance: `cargo package --list` shows license files; CI green on the tag.
- [ ] M6.5 Launch checklist: Show HN, r/linux, r/unixporn, r/rust, compositor subreddits, AUR package. Acceptance: a checklist file with final post texts, ready for the user to submit.
