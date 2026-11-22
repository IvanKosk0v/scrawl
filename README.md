# scrawl

Mouse-stroke gestures for **any** Linux Wayland compositor (and X11). Hold a mouse button, draw a stroke, and
scrawl runs the action you bound to it: a key combo or a command. Plain clicks, including right-click menus, keep working.

<!-- demo-gif -->
![scrawl demo: back/forward and tab strokes in a browser, a right-click menu still working, then the escape chord](docs/demo.gif)

- **Every compositor, and X11.** It reads the mouse through evdev and re-emits it through uinput, so it needs no compositor API.
- **Clicks still work.** A press that does not turn into a stroke is replayed as an ordinary click. Use a side or middle button as the trigger if you prefer.
- **No `ydotool`/`xdotool`, no root daemon.** Key actions go through its own virtual keyboard; a udev rule gives your seat user access instead of running anything as root.
- **Safe by design.** An escape chord, `--timeout`, and release of the grab on Ctrl-C, SIGTERM or a crash.
- **Easy to set up.** `record` shows the stroke you just drew, a TOML config is checked by `check-config` and reloads when saved, with per-device and per-app profiles.

## Install
Debian/Ubuntu: download the `.deb` from the GitHub release and install it (the package reloads the udev rule):

```sh
sudo apt install ./scrawl_*.deb
```

Arch/Artix: build the package from the tag tarball (needs `cargo`; installs the udev rule and the user unit):

```sh
cd packaging/arch && makepkg -si
```

From source (Rust, see `rust-version` in `Cargo.toml`): a per-user install that never uses root and prints the one
root step (the udev rule) for you to run:

```sh
cargo build --release
./packaging/install.sh
```

Then log out and in once so the udev rule applies to your session.

## Migrating from strokebind

Version 0.1.0 was called strokebind. scrawl does not read the old files, so move them once
(`scrawl check-config ~/.config/scrawl/config.toml` prints the `mv` command if the old config is still there):

```sh
systemctl --user disable --now strokebind.service
mv ~/.config/strokebind ~/.config/scrawl                      # config dir
rm ~/.config/systemd/user/strokebind.service ~/.local/bin/strokebind
./packaging/install.sh                                         # installs scrawl.service, keeps your config
sudo rm /etc/udev/rules.d/70-strokebind.rules                  # old udev rule (install.sh prints the new one)
systemctl --user enable --now scrawl.service
```

With the `.deb`, remove the old package (`sudo apt remove strokebind`) and install `scrawl_*.deb`; it ships
`scrawl.service` and `70-scrawl.rules`. picom: replace `class_g = 'strokebind-overlay'` with
`class_g = 'scrawl-overlay'` in your exclude lists or `rules`.

## Quick start
```sh
scrawl list-devices                       # find your mouse
scrawl record                             # draw strokes; prints e.g. `L-U` after each release (no grab)
scrawl check-config ~/.config/scrawl/config.toml
scrawl run --config ~/.config/scrawl/config.toml --timeout 30   # try it with a 30 s safety net
systemctl --user enable --now scrawl.service                        # run it at login
```
`record` never grabs the mouse. `run` does, so keep `--timeout` on while trying a new config.

## Configuration
Strokes are `U D L R UL UR DL DR` joined by `-` (`L-U` = left, then up). The file lives at
`~/.config/scrawl/config.toml`; `install.sh` installs this starter:

```toml
trigger = "right"

[[binding]]
stroke = "L"
action = { key = "alt+left" }

[[binding]]
stroke = "R"
action = { key = "alt+right" }
```

More: commands, per-app bindings, per-device profiles and notifications.

```toml
trigger = "right"
min_stroke_px = 40          # shorter movements are clicks or wiggles
escape = "left+right+middle"
feedback = "notify"         # show each recognized stroke via notify-send
overlay = true              # draw the stroke trail while the trigger is held

[[binding]]
stroke = "DR"
action = { run = ["notify-send", "hello"] }   # argv, no shell

[[binding]]
stroke = "L"
app = "firefox"             # WM_CLASS or app_id, case-insensitive
action = { key = "ctrl+shift+tab" }

[[profile]]
app = "kitty"

[[profile.binding]]
stroke = "R"
action = { key = "ctrl+shift+t" }

[[profile]]
device = "trackball"        # part of the device name
trigger = "middle"
```

Edits to bindings are picked up while `run` is running (a broken edit is reported and ignored); changing the trigger,
`min_stroke_px` or `escape` needs a restart. By default the pointer keeps moving while you draw; `freeze_pointer = true`
holds it still, and a release within `click_slop_px` (default 6) of the press counts as a click.

## Support matrix
Strokes and clicks work on every desktop, because scrawl reads the mouse through evdev. Per-app bindings
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
`scrawl run` grabs your mouse exclusively, so there are several ways out that never need the mouse to work:
- **Escape chord:** hold all of its buttons at once to exit. The default is `left+right+middle`. Set it in the
  config with `escape = "side+extra"` (at least two distinct buttons from left, right, middle, side and extra).
  Only the mouse is read; scrawl never opens your keyboard.
  `run` refuses to start if the mouse lacks a chord button, so the escape always works.
- **`--timeout N`:** exit by itself after N seconds (N >= 1). Use this when trying a new config.
- **Ctrl-C / SIGTERM, or a crash:** the process exits and the kernel releases the grab.

## Comparison
| Tool | Works on | Notes |
|---|---|---|
| **scrawl** | every Wayland compositor and X11 | device-level; per-app bindings on X11, Sway, Hyprland |
| Easystroke | X11 only | discontinued |
| wstroke | wlroots compositors only | |
| Input Actions | KDE Plasma only | |
| mouse-actions | X11, limited on Wayland | alpha |

Based on a survey of these projects in November 2026. scrawl does not remap keys (see kanata, xremap) or handle
touchpad gestures (see libinput-gestures).

## FAQ
**Does a normal right-click still work?** Yes. A press that does not become a stroke is replayed as a click, so context menus open as usual.

**Why does it need access to `/dev/uinput`?** It re-emits your mouse through a virtual device and types key combos through a virtual keyboard. The udev rule grants this to the user at your seat; the daemon never runs as root.

**What if the mouse gets stuck?** Hold the escape chord (default left+right+middle), stop the service with `systemctl --user stop scrawl`, or press Ctrl-C. The kernel releases the grab when the process exits.

**Why are per-app bindings missing on KDE and GNOME?** scrawl has no focused-window adapter for them yet. Global bindings work there; per-app support is a 0.1 limitation.

**Where does the trail (`overlay = true`) work?** On X11 and on Wayland compositors with wlr-layer-shell (Sway, Hyprland, KDE). The trail is click-through. On Wayland it starts at the center of the focused monitor (there is no global cursor position) and is drawn at that monitor's scale; on X11 one window covers every monitor and the trail follows the real cursor. On Wayland the trail is built from raw mouse motion, so with pointer acceleration it can drift from the cursor on fast moves (known limit). GNOME has no layer-shell, so the trail is unavailable there and scrawl prints `overlay disabled`; use `feedback = "notify"` instead. Changing `overlay` needs a restart.

**picom animates or shadows the trail. How do I stop that?** Exclude the overlay window by its class. For picom up to v11, add `class_g = 'scrawl-overlay'` to `animation-exclude`, `fade-exclude` and `shadow-exclude`. For picom v12 and later, use a `rules` entry with `match = "class_g = 'scrawl-overlay'"` that turns these effects off.

**Can I keep my keyboard shortcuts?** Key actions use a virtual keyboard, and scrawl never reads your real keyboard.

## License
Dual-licensed under MIT or Apache-2.0, at your option.
