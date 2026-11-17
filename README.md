# strokebind

Mouse-stroke gestures for **any** Linux Wayland compositor (and X11). Draw a stroke while holding a mouse button;
strokebind runs the action you bound to it. Plain clicks (including right-click menus) keep working.

> Status: pre-alpha, under construction. See PLAN.md. This README is rewritten in milestone M6.

## Failsafes
`strokebind run` grabs your mouse exclusively, so there are several ways out that never need the mouse to work:
- **Escape chord:** hold all of its buttons at once to exit. The default is `left+right+middle`. Set it in the
  config with `escape = "side+extra"` (at least two distinct buttons from left, right, middle, side and extra).
  Only the mouse is read; strokebind never opens your keyboard.
- **`--timeout N`:** exit by itself after N seconds (N >= 1). Use this when trying a new config.
- **Ctrl-C / SIGTERM, or a crash:** the process exits and the kernel releases the grab.

## License
Dual-licensed under MIT or Apache-2.0, at your option.
