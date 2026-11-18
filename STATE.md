# STATE (keep under 40 lines; Claude updates, user may read)

Current milestone: M5
Current task: M5.4
Branch: m5-adapters
Last session: M5.3 done (2026-11-18)

## Handoff
Done: M5.3 `focus::Focused` wraps the engine, samples the adapter when a trigger press starts pending; `forward` passes the press-time app to `run`, which uses `bindings_for(app)`. `run` uses `GlobalOnly` until M5.4. M5.2 `app` config done.
Next: M5.4 backend picker (WAYLAND_DISPLAY before DISPLAY), replaces `GlobalOnly` in main.rs.
Fragile: adapter is called synchronously in the input loop at each trigger press, so M5.5-M5.7 need short IPC timeouts; no Wayland run yet (M6.0); key-combo action not in the user's hardware list. Reload polled only on strokes. Compensating release after SYN_DROPPED arrives on next device event. Deferred LOWs: grab before virtual mouse creation; button held at grab time.

## Decisions
- Rust; deps limited to docs/ARCHITECTURE.md allowed list.
- evdev grab + uinput forwarding so it works on every compositor.
- Dual license MIT OR Apache-2.0.
- MSRV 1.85.1: lowest passing version found by `cargo msrv find --min 1.70` (bisect; older toolchains fail on a dependency needing edition 2024).
- Review of M0: fixed unknown-field typos (deny_unknown_fields). Accepted: gates.rs checks strings only; CI msrv+deny jobs are the real verification.
- M1: jitter filter skips points < min_len from last accepted; noise runs dropped then neighbours re-merged; tolerance = max total 45-degree steps, ties (incl. duplicate bindings) = Ambiguous; proptests use exact-grid paths (20px steps, jitter +-2).
- M2.1: opens nodes with evdev Device::open (read-only, no grab); unreadable-only => PermissionDenied error with input-group/udev hint; other open failures skipped. Removed device::MODULE placeholder; layout test now references list_mice_in.
- M2.2: no EventSource trait; the loop takes any iterator of Result<InputEvent>. Unfinished stroke at end of stream is discarded; LoopParams::from_config: min_run = min_stroke_px, min_step = min_stroke_px/4.
- M2.3: ReportMapper folds each SYN_REPORT into one Motion (first) then button changes; ignores autorepeat and non-trigger buttons. record defaults: first mouse, Right trigger, 40px.
- M3.1: abs axes not mirrored; virtual name `strokebind: <src>` cut to 78 bytes (evdev build() asserts <80); real-uinput test is #[ignore], run with `cargo test --test virtual_mouse -- --ignored`.
- M3.2: release model = kernel drops grab on fd close; panic (any thread) and SIGINT/SIGTERM exit the process (101/130/143) instead of unwinding. Relies on O_CLOEXEC (tested) so M4.2 children never inherit the grab. signal-hook 0.3 (allowed list). Grab tests are #[ignore] (uinput): `cargo test --test grab -- --ignored`.
- M3.3: decided at trigger release via recognize(): None = replay all buffered reports verbatim (incl. small jiggle); Some = drop buffered REL_X/Y, trigger, MSC, re-emit the rest (wheel, other buttons). SYN_DROPPED: evdev sync Device re-injects lost state, engine only discards its partial report (no resync API).
- M3.4 (user chose): escape = mouse-button chord only (never read keyboards); >=2 distinct buttons; checked on every report before passthrough logic, wins even while trigger pending. --timeout >=1 s, thread then exit(0). CLI stub test now uses check-config (run implemented).
- M3 review: MED1 fixed by refusing to run with an escape chord the device lacks (EscapeUnavailable). MED2: synced evdev Device never yields SYN_DROPPED; it injects compensating key events on the next fetch, proven by `trigger_release_lost_in_overflow_is_recovered`. So no engine change; the engine's SYN_DROPPED branch is defensive only. LOWs deferred.
- approved P2: `feedback = "notify"` as M4.5 (no new deps). approved P3: M4.6 live-motion default (`freeze_pointer = false`, `click_slop_px = 6`, between-band swallowed); M3.3 now = freeze_pointer=true mode. Caveat: focus-follows-mouse may shift focus during live strokes.
- M4.1: key names = aliases (ctrl/control, shift, alt, altgr, super/meta/win/logo, return, escape, del, ins, pgup, pgdn, period, print) or any evdev KEY_* name without prefix, case-insensitive. Only the last key may be a non-modifier; no duplicates; BTN_* rejected. One key event per report; modifiers released in reverse. Virtual keyboard declares 1..=0xff and 0x160..=0x2bf. Config `Action::Key` stays a String until M4.3/M4.4 validate it. The uinput test types shift+F24.
- M4.2: no shell (argv direct; opt in via ["sh","-c",..]); env allowlist HOME USER LOGNAME SHELL PATH LANG LANGUAGE TZ DISPLAY XAUTHORITY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS DESKTOP_SESSION SWAYSOCK I3SOCK HYPRLAND_INSTANCE_SIGNATURE + LC_* + XDG_*; PATH default if missing; cwd $HOME or /; stdin+stdout null, stderr inherited; reaped by a thread (no unsafe/setsid).
- approved P1: stroke trail overlay added as post-0.1 milestone M7 (5 tasks, branch m7-overlay); deps x11rb, smithay-client-toolkit, tiny-skia allowed for M7 only.
- M4.3: profile = `[[profile]] device = "<name substring, case-insensitive>"`, first match wins, `[[profile.binding]]` beat global ones per stroke. Hot reload (notify 8, parent dir watched, polled when a stroke arrives) swaps bindings only; trigger/min_stroke_px/escape need a restart (message printed). Match tolerance fixed at 0. Key combos / empty run argv rejected at load via serde try_from.
- M4.4: `check-config <FILE>` positional (no default path yet). OK -> stdout `<path>: OK (N bindings, M profiles)` exit 0; error -> stderr `error: <path> line L: msg` + `  L | <line>`, exit 1 (process::exit, no anyhow prefix). Multi-line toml messages joined with `; `. Missing top-level field reports no line (toml blames root table at offset 0). Goldens: `ok_*`/`err_*.toml` + `.out`. Removed dead `Error::NotImplemented`/`Command::name`.
- M4.5: `feedback` is global only (copied into Resolved, so hot reload applies it). Fires only for recognized strokes (clicks never notify); no binding/ambiguous = `<stroke>: no match`. argv `notify-send --app-name=strokebind --expire-time=1500 strokebind <body>` via spawn_detached; runs after the action, failures only printed.
- approved P4: M7.1 backend picker checks WAYLAND_DISPLAY before DISPLAY (XWayland); selection test added. approved P5: M6.0 compositor verification (Sway, GNOME, KDE, Hyprland, X11 -> README matrix) before M6.1.
- M4.6: engine default stays Frozen (M3.3 tests unchanged = true-mode proof); Live withholds trigger+MSC of the press report, emits the rest live; at release: rest of report first, then max dist from press <= slop (checked first) = press+release replayed, else stroke, else swallowed. freeze/slop global only, restart-only. Review fixes: slop check skipped when frozen; error line = offending profile's `min_stroke_px`, else top-level `click_slop_px`/`min_stroke_px` (section-scoped search); repeated trigger press while held is stripped; dist² saturates.
- M4 review: keyboard created eagerly via `Actions::prepare` (startup failure = exit, reload failure printed; a reload-time creation can still lose that same stroke's keys since reload is polled per stroke). Blank profile `device` rejected. LOWs deferred: reload polled only on strokes; partial key-combo emit failure may leave modifiers held.
- M5.1: `active_window` returns Option (failure = None = global bindings; adapter reports its own error); `AppId::new` trims, blank = None; `GlobalOnly` always None (for M5.3 unsupported desktops). Adapters query fresh each call, no caching.
- approved P6: M5 = 8 tasks (app config, run wiring at press, backend picker, X11/Sway/Hyprland adapters, README matrix; KDE+GNOME global-only for 0.1). Deps: serde_json new, x11rb extended to M5; no zbus.
- M5.2: app match = whole id, case-insensitive (stored trimmed+lowercased). `[[profile]]` needs `device` or `app`; `app` profile = bindings only (trigger/min_stroke_px/escape rejected), fits every mouse unless `device` also set (both must match); `app` on an app profile's binding rejected. Precedence per stroke: device-profile app binding > app profiles (file order) > global app binding > device-profile binding > global. `Resolved.bindings` = all candidates (keyboard prepare), `bindings_for(None)` = no-app only. Profile errors via serde try_from point at the `[[profile]]` header.
- M5.3: no new engine Output; `Passthrough::is_pending()` + `Focused` wrapper detect press (pending off->on), one adapter call per press (clicks too), none for motion/other buttons. Press app kept until the next press. `forward` callback = `(stroke, Option<AppId>)`.
- Stroke grammar: U,D,L,R,UL,UR,DL,DR joined by `-` (`L-U` = left then up). Config: `trigger` required, `min_stroke_px` defaults to 40, errors carry 1-based line.

## Proposals (numbered; awaiting user `/approve P<n>` or `/reject P<n>`)
