# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M1
Current task: M1.3
Branch: m1-recognizer
Last session: M1.2 done

## Handoff
Done: M1.2 recognizer::stroke::stroke_string(segments, min_run_len) -> Option<String>; Direction::as_str.
Next: M1.3 matcher stroke string -> binding with configurable tolerance.
Fragile: stroke_string returns plain String, not config::Stroke; tolerance semantics for M1.3 undecided. Push + CI not yet run.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.
- MSRV 1.85.1: lowest passing version found by `cargo msrv find --min 1.70` (bisect; older toolchains fail on a dependency needing edition 2024).
- Review of M0: fixed unknown-field typos (deny_unknown_fields). Accepted: gates.rs checks strings only; CI msrv+deny jobs are the real verification.
- M1.1: jitter filter = skip points within min_len of last accepted point; per-step segments, no merging; y grows down, ties snap via atan2 round.
- M1.2: noise runs dropped after merging, then neighbours re-merged (R,tiny U,R -> R); no stroke = None.
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
