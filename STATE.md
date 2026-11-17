# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M1
Current task: M1.2
Branch: m1-recognizer
Last session: M1.1 done

## Handoff
Done: M1.1 recognizer::segment (Point, Direction, Segment, segments()); proptest added as dev-dep.
Next: M1.2 segments -> stroke string (merge repeats, drop tiny); may reuse config::Stroke.
Fragile: segments() does not merge same-direction steps; M1.2 must. Push + CI not yet run.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.
- MSRV 1.85.1: lowest passing version found by `cargo msrv find --min 1.70` (bisect; older toolchains fail on a dependency needing edition 2024).
- Review of M0: fixed unknown-field typos (deny_unknown_fields). Accepted: gates.rs checks strings only; CI msrv+deny jobs are the real verification.
- M1.1: jitter filter = skip points within min_len of last accepted point; per-step segments, no merging; y grows down, ties snap via atan2 round.
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
