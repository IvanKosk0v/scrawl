# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M1
Current task: M1.1
Branch: (create `m1-recognizer` from main on first /next)
Last session: M0 closed

## Handoff
Done: M0 Foundation merged to main (layout, CLI stubs, config parse, MSRV 1.85.1, deny).
Next: M1.1 point stream -> 8-direction segments (pure logic, proptest).
Fragile: Stroke type lives in config.rs; M1.2 may need it in recognizer. Push + CI not yet run.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.
- MSRV 1.85.1: lowest passing version found by `cargo msrv find --min 1.70` (bisect; older toolchains fail on a dependency needing edition 2024).
- Review of M0: fixed unknown-field typos (deny_unknown_fields). Accepted: gates.rs checks strings only; CI msrv+deny jobs are the real verification.
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
