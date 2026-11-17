# Architecture (decisions are fixed unless the user changes them)

## Data flow
mouse (evdev, exclusively grabbed) -> event loop -> [trigger button held?]
  - no: forward event unchanged to uinput virtual mouse
  - yes: collect REL_X/REL_Y points -> recognizer -> stroke string
    - plain click (movement below threshold): replay press+release unchanged
    - stroke: look up binding -> action (key combo via uinput keyboard, or command)

## Why this design
Compositor-independent: no Wayland protocol needed for input or actions.
Per-app profiles are optional adapters that only read the active window; failure falls back to global bindings.

## Modules
- `recognizer/` pure logic, no I/O, fully unit-tested
- `device/` evdev enumeration + event source trait (real + fake for tests)
- `sys/` all `unsafe` / raw syscalls, each use documented
- `actions/` key combo, run command
- `config.rs` TOML types and validation

## Safety rules for grab code
- Grab released on Drop, panic hook, SIGINT, SIGTERM.
- `--timeout N` and a failsafe key combo exit the daemon.
- Never grab a keyboard in v0.x.

## Allowed dependencies (anything else needs user approval via STATE.md Proposals)
evdev, clap (derive), serde, toml, thiserror, anyhow, notify (hot reload), signal-hook, log + env_logger.
Dev: tempfile, proptest.

## Config sketch (~/.config/strokebind/config.toml)
```toml
trigger = "right"          # right | middle | side | extra
min_stroke_px = 40
escape = "left+right+middle"  # hold all to exit run (failsafe)

[[binding]]
stroke = "L"
action = { key = "alt+left" }      # back

[[binding]]
stroke = "DR"
action = { run = ["notify-send", "hello"] }
```
