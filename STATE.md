# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M4
Current task: M4.1
Branch: main (create m4-actions from main before M4.1)
Last session: M3 closed

## Handoff
Done: M3 merged to main (not pushed). User verified `run --timeout 20` on a real mouse: movement, clicks, right-click menu, strokes, timeout OK.
Next: M4.1 key combo through a uinput virtual keyboard (key-name parsing + emitted sequence tests).
Fragile: pointer frozen while trigger held (by design). Compensating release after SYN_DROPPED arrives on next device event. Deferred LOWs: grab before virtual mouse creation; button held at grab time.

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
- M3.3: decided at trigger release via recognize(): None = replay all buffered reports verbatim (incl. small jiggle); Some = drop buffered REL_X/Y, trigger, MSC, re-emit the rest (wheel, other buttons). SYN_DROPPED: evdev sync Device re-injects lost state, engine only discards its partial report (no resync API).
- M3.4 (user chose): escape = mouse-button chord only (never read keyboards); >=2 distinct buttons; checked on every report before passthrough logic, wins even while trigger pending. --timeout >=1 s, thread then exit(0). CLI stub test now uses check-config (run implemented).
- M3 review: MED1 fixed by refusing to run with an escape chord the device lacks (EscapeUnavailable). MED2: synced evdev Device never yields SYN_DROPPED; it injects compensating key events on the next fetch, proven by `trigger_release_lost_in_overflow_is_recovered`. So no engine change; the engine's SYN_DROPPED branch is defensive only. LOWs deferred.
- approved P2: `feedback = "notify"` added as M4.5 (notify-send via M4.2 path, no new deps).
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
- P1 (IDEA, revised) Stroke trail overlay behind an `Overlay` trait (2 real backends, so not speculative). X11: transparent click-through override-redirect window (bspwm). Wayland: wlr-layer-shell (Sway, Hyprland, KDE). GNOME: none, uses P2 notify. Backend picked from WAYLAND_DISPLAY/DISPLAY. New deps: x11rb (shape+xfixes for the empty input region; ARGB transparency needs a compositor such as picom, else a Shape-masked trail); smithay-client-toolkit (pulls wayland-client, wayland-protocols, wayland-protocols-wlr, calloop); tiny-skia (shared drawing). Cost: very high, its own milestone of ~5 tasks: trait + renderer, X11, Wayland, a render thread fed by the event loop, multi-output/HiDPI. Blocker: we only see relative motion, so the trail needs the cursor position at press. X11 gets it from QueryPointer; Wayland has no global cursor API (Hyprland IPC only), so elsewhere the trail would start at screen center. Benefit: easystroke-like feedback. Suggest a post-0.1 milestone.
- P3 (IDEA, revised 2) New config `freeze_pointer` (default false) + `click_slop_px` (default 6, must be < min_stroke_px). false (live): REL_X/Y forwarded immediately while the trigger is held; only the trigger press is withheld. At release, measured by max distance from the press point: <= click_slop_px = click, emit press+release (MSC kept) at the release position; a recognized stroke = action, no click; between slop and a recognized stroke = swallowed (no click, no action). true: today's M3.3 behavior unchanged (motion frozen, no stroke = verbatim replay; slop not used). Changes M3.3 "replayed unchanged" for live mode: the click is rebuilt, not replayed verbatim, and press-move-return beyond the slop is swallowed. M3.3 tests stay valid for true; live mode gets new tests (click within slop, stroke, swallow band, return-beyond-slop). Caveat: with focus-follows-mouse (Sway/Hyprland/bspwm) a live stroke can move focus before the action fires. No new deps. Cost: medium (engine mode, config validation, ~6 tests); plan change = new task in M4. Benefit: cursor feedback while drawing.
