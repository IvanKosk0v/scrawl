# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M0
Current task: M0.3
Branch: m0-foundation
Last session: M0.2 done

## Handoff
Done: M0.2 clap CLI (src/cli.rs), stubs exit non-zero with "not implemented"; clap+anyhow added.
Next: M0.3 config types + TOML parse (needs serde and toml deps, both on allowed list).
Fragile: module MODULE consts are placeholders; Error::Unsupported only used by a test.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
