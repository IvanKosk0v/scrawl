# strokebind

Mouse-stroke gestures for **any** Linux Wayland compositor (and X11). Draw a stroke while holding a mouse button;
strokebind runs the action you bound to it. Plain clicks (including right-click menus) keep working.

> Status: pre-alpha, under construction. See PLAN.md. This README is rewritten in milestone M6.

## Support matrix
Strokes and clicks work on every desktop, because strokebind reads the mouse through evdev. Per-app bindings
(`app = "..."`) need to know the focused window, which depends on the desktop:

| Desktop | Per-app bindings | How the focused app is read | Verified on a real session |
|---|---|---|---|
| X11 (any window manager) | yes | EWMH `_NET_ACTIVE_WINDOW` + `WM_CLASS` | yes (bspwm) |
| Sway | yes | i3 IPC `get_tree` over `SWAYSOCK` | yes |
| Hyprland | yes | `j/activewindow` over the socket named by `HYPRLAND_INSTANCE_SIGNATURE` | yes |
| KDE Plasma (Wayland) | global bindings only | not available in 0.1 | yes |
| GNOME (Wayland) | global bindings only | not available in 0.1 | yes |

"Verified" means `record` and `run` were checked by hand on that desktop: clicks, strokes, run and key-combo actions, and
(where supported) per-app bindings.

## Failsafes
`strokebind run` grabs your mouse exclusively, so there are several ways out that never need the mouse to work:
- **Escape chord:** hold all of its buttons at once to exit. The default is `left+right+middle`. Set it in the
  config with `escape = "side+extra"` (at least two distinct buttons from left, right, middle, side and extra).
  Only the mouse is read; strokebind never opens your keyboard.
  `run` refuses to start if the mouse lacks a chord button, so the escape always works.
- **`--timeout N`:** exit by itself after N seconds (N >= 1). Use this when trying a new config.
- **Ctrl-C / SIGTERM, or a crash:** the process exits and the kernel releases the grab.

## License
Dual-licensed under MIT or Apache-2.0, at your option.
