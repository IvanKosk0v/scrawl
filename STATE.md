# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M0
Current task: none (M0 complete, run /milestone)
Branch: m0-foundation
Last session: M0.4 done

## Handoff
Done: M0.4 MSRV measured 1.85.1 (cargo msrv find), cargo deny passes and runs in check.sh, CI unchanged.
Next: /milestone to close M0, then M1.1.
Fragile: deny.toml has unused license allowances (warnings only); MSRV must be re-measured when deps are added.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.
- MSRV 1.85.1: cargo-msrv bisect from 1.70; 1.85.0 fails (edition 2024 in a dependency).
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
