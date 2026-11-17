# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M3
Current task: M3.3
Branch: m3-grab
Last session: M3.2 done

## Handoff
Done: M3.2 `device::grab` (Grabbed: grab/device_mut, ungrab on Drop; install_failsafes: panic hook exit 101, signal thread exit 128+sig). tests/grab.rs child-process tests.
Next: M3.3 click passthrough (hold press until stroke-vs-click decided; replay plain click). Wire Grabbed + VirtualMouse.
Fragile: SYN_DROPPED can drop a buffered trigger release, so resync before grab. uinput node needs udev for a moment after creation (tests retry open).

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
- M2.1: opens nodes with evdev Device::open (read-only, no grab); unreadable-only => PermissionDenied error with input-group/udev hint; other open failures skipped. Removed device::MODULE placeholder; layout test now references list_mice_in.
- M2.2: no EventSource trait; the loop takes any iterator of Result<InputEvent>. Unfinished stroke at end of stream is discarded; LoopParams::from_config: min_run = min_stroke_px, min_step = min_stroke_px/4.
- M2.3: ReportMapper folds each SYN_REPORT into one Motion (first) then button changes; ignores autorepeat and non-trigger buttons. record defaults: first mouse, Right trigger, 40px.
- M3.1: abs axes not mirrored; virtual name `strokebind: <src>` cut to 78 bytes (evdev build() asserts <80); real-uinput test is #[ignore], run with `cargo test --test virtual_mouse -- --ignored`.
- M3.2: release model = kernel drops grab on fd close; panic (any thread) and SIGINT/SIGTERM exit the process (101/130/143) instead of unwinding. Relies on O_CLOEXEC (tested) so M4.2 children never inherit the grab. signal-hook 0.3 (allowed list). Grab tests are #[ignore] (uinput): `cargo test --test grab -- --ignored`.
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
(none)
