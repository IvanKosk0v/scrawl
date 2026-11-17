# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M4
Current task: M4.3
Branch: m4-actions
Last session: M4.2 done

## Handoff
Done: M4.2 `actions::command` (sanitized_env allowlist, build_command, spawn_detached with reaper thread, current_env). Error::EmptyCommand/Spawn. Not wired to run yet.
Next: M4.3 bindings from config (validate key combos + argv at load), per-device profiles, hot reload (notify dep allowed); wire actions into `run`.
Fragile: frozen pointer until M4.6. Compensating release after SYN_DROPPED arrives on next device event. Deferred LOWs: grab before virtual mouse creation; button held at grab time.

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
- approved P3: M4.6 live-motion default (`freeze_pointer = false`, `click_slop_px = 6`, between-band swallowed); M3.3 now = freeze_pointer=true mode. Caveat: focus-follows-mouse may shift focus during live strokes.
- M4.1: key names = aliases (ctrl/control, shift, alt, altgr, super/meta/win/logo, return, escape, del, ins, pgup, pgdn, period, print) or any evdev KEY_* name without prefix, case-insensitive. Only the last key may be a non-modifier; no duplicates; BTN_* rejected. One key event per report; modifiers released in reverse. Virtual keyboard declares 1..=0xff and 0x160..=0x2bf. Config `Action::Key` stays a String until M4.3/M4.4 validate it. The uinput test types shift+F24.
- M4.2: no shell (argv direct; opt in via ["sh","-c",..]); env allowlist HOME USER LOGNAME SHELL PATH LANG LANGUAGE TZ DISPLAY XAUTHORITY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS DESKTOP_SESSION SWAYSOCK I3SOCK HYPRLAND_INSTANCE_SIGNATURE + LC_* + XDG_*; PATH default if missing; cwd $HOME or /; stdin+stdout null, stderr inherited; reaped by a thread (no unsafe/setsid).
- Stroke grammar: 8 tokens U,D,L,R,UL,UR,DL,DR joined by `-` (so `DR` = down-right diagonal, `L-U` = left then up).
- Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
- P1 (IDEA, revised) Stroke trail overlay behind an `Overlay` trait (2 real backends, so not speculative). X11: transparent click-through override-redirect window (bspwm). Wayland: wlr-layer-shell (Sway, Hyprland, KDE). GNOME: none, uses P2 notify. Backend picked from WAYLAND_DISPLAY/DISPLAY. New deps: x11rb (shape+xfixes for the empty input region; ARGB transparency needs a compositor such as picom, else a Shape-masked trail); smithay-client-toolkit (pulls wayland-client, wayland-protocols, wayland-protocols-wlr, calloop); tiny-skia (shared drawing). Cost: very high, its own milestone of ~5 tasks: trait + renderer, X11, Wayland, a render thread fed by the event loop, multi-output/HiDPI. Blocker: we only see relative motion, so the trail needs the cursor position at press. X11 gets it from QueryPointer; Wayland has no global cursor API (Hyprland IPC only), so elsewhere the trail would start at screen center. Benefit: easystroke-like feedback. Suggest a post-0.1 milestone.
