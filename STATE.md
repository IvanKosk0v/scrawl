# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M1
Current task: M1.4
Branch: m1-recognizer
Last session: M1.3 done

## Handoff
Done: M1.3 recognizer::matcher::match_stroke(stroke, bindings, tolerance) -> MatchResult; Direction::{from_token,steps_to}.
Next: M1.4 end-to-end proptests (points -> stroke): no panic, scale/jitter invariance, empty/one point = none.
Fragile: tolerance is not yet in Config (needs a field when wired in M4.3); no pipeline fn composing segments+stroke_string yet.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.
- MSRV 1.85.1: lowest passing version found by `cargo msrv find --min 1.70` (bisect; older toolchains fail on a dependency needing edition 2024).
- Review of M0: fixed unknown-field typos (deny_unknown_fields). Accepted: gates.rs checks strings only; CI msrv+deny jobs are the real verification.
- M1.1: jitter filter = skip points within min_len of last accepted point; per-step segments, no merging; y grows down, ties snap via atan2 round.
- M1.2: noise runs dropped after merging, then neighbours re-merged (R,tiny U,R -> R); no stroke = None.
- M1.3: tolerance = max total 45-degree steps (circular) across equal-length strokes; ties for best distance (incl. duplicate bindings) = Ambiguous, no action.
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
