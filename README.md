# strokebind

Mouse-stroke gestures for **any** Linux Wayland compositor (and X11). Draw a stroke while holding a mouse button;
strokebind runs the action you bound to it. Plain clicks (including right-click menus) keep working.

> Status: pre-alpha, under construction. See PLAN.md. This README is rewritten in milestone M6.

## Support matrix
Strokes and clicks work on every desktop, because strokebind reads the mouse through evdev. Per-app bindings
(`app = "..."`) need to know the focused window, which depends on the desktop:

| Desktop | Per-app bindings | How the focused app is read |
|---|---|---|
| X11 (any window manager) | yes | EWMH `_NET_ACTIVE_WINDOW` + `WM_CLASS` |
| Sway | yes | i3 IPC `get_tree` over `SWAYSOCK` |
| Hyprland | yes | `j/activewindow` over the socket named by `HYPRLAND_INSTANCE_SIGNATURE` |
| KDE Plasma (Wayland) | global bindings only | not available in 0.1 |
| GNOME (Wayland) | global bindings only | not available in 0.1 |

Real-session checks so far: KDE Plasma Wayland and X11 with bspwm. The Sway and Hyprland adapters are tested
against recorded replies and a fake socket only; GNOME is untested. Details land in M6.0.

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
