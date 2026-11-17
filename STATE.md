# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M0
Current task: M0.2
Branch: m0-foundation
Last session: M0.1 done

## Handoff
Done: M0.1 layout (lib.rs, error, config, recognizer/device/sys stubs), thiserror added.
Next: M0.2 clap CLI skeleton (clap not yet in Cargo.toml; it is on the allowed list).
Fragile: module `MODULE` consts are placeholders; replace when modules get real content.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
