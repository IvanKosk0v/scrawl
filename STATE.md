# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M0
Current task: M0.4
Branch: m0-foundation
Last session: M0.3 done

## Handoff
Done: M0.3 Config::parse (serde+toml), Stroke/Trigger/Action types, Error::Config with line.
Next: M0.4 real MSRV via cargo-msrv, cargo deny passing, check.sh runs it.
Fragile: Stroke lives in config.rs; M1.2 may move it to recognizer. layout.rs no longer calls Config::default().

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
