# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M1
Current task: none (M1 complete, run /milestone)
Branch: m1-recognizer
Last session: M1.4 done

## Handoff
Done: M1.4 recognizer::recognize pipeline + tests/recognizer_props.rs (256 cases each).
Next: /milestone to close M1, then M2.1 (device layer, no grab).
Fragile: tolerance not yet a Config field; recognize() params (min_step, min_run) not yet wired to config min_stroke_px.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.
- MSRV 1.85.1: lowest passing version found by `cargo msrv find --min 1.70` (bisect; older toolchains fail on a dependency needing edition 2024).
- Review of M0: fixed unknown-field typos (deny_unknown_fields). Accepted: gates.rs checks strings only; CI msrv+deny jobs are the real verification.
- M1.1: jitter filter = skip points within min_len of last accepted point; per-step segments, no merging; y grows down, ties snap via atan2 round.
- M1.2: noise runs dropped after merging, then neighbours re-merged (R,tiny U,R -> R); no stroke = None.
- M1.3: tolerance = max total 45-degree steps (circular) across equal-length strokes; ties for best distance (incl. duplicate bindings) = Ambiguous, no action.
- M1.4: scaling/jitter proptests use exact-grid paths (20px steps, jitter +-2) so expectations are deterministic.
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
