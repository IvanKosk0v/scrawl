# Research summary (Nov 2026). Read only for README / launch work.

## Why this project
- Easystroke (X11 mouse gestures) is discontinued; users ask for a Wayland replacement.
- A KDE forum thread on Plasma 6 Wayland mouse gestures had users pooling a ~$1,500 bounty.
- No existing tool works on every compositor: wstroke = wlroots only, Input Actions = Plasma only (~370 stars),
  mouse-actions = alpha with Wayland limits.
- AutoHotkey-style scripting on Wayland is fragmented (ydotool needs root/uinput, kdotool KDE-only, wdotool new).

## Adjacent star counts (ceiling guide, not a forecast)
input-remapper ~5.5k, keymapper ~4.9k, libinput-gestures ~4k, xremap ~2.1k, evremap ~620.
Realistic target for this project: hundreds to low thousands.

## Differentiators to claim in README (only once actually implemented and tested)
1. Works on every Wayland compositor (and X11): device-level, no compositor API.
2. Plain right-click still works (click passthrough); or pick side/middle button as trigger.
3. No ydotool/xdotool dependency; no root-run daemon.
4. Safe by design: grab released on crash/exit; failsafe + timeout.
5. `record` mode to discover strokes; TOML config with hot reload; per-device profiles.
6. Per-app profiles via optional adapters (Sway, Hyprland, best-effort KDE/GNOME); global fallback always works.
Do NOT claim: key remapping (kanata/xremap), touchpad gestures (libinput-gestures), plain key injection (wdotool).

## Launch tactics that worked for similar tools
Show HN with a README that sells in 10 seconds, demo GIF, single-command install; reddit r/linux, r/unixporn,
r/rust, compositor subreddits; respond to every comment in the first day.
