# strokebind 0.1.0 launch checklist

Final texts, ready to paste. Every claim below is backed by the README, the tests or the manual verification list in STATE.md.
Nothing here has been posted. Tick a box only after you did it.

## Before posting
- [ ] `v0.1.0` release exists with the `.deb` attached: https://github.com/IvanKosk0v/strokebind/releases/tag/v0.1.0
- [ ] The milestone PR is merged into `main`, so the README and `docs/demo.gif` render on the default branch page.
- [ ] Repo description and topics are set (`wayland`, `mouse-gestures`, `evdev`, `linux`, `rust`), Issues are enabled, private vulnerability reporting is on.
- [ ] You tried the install path once on a clean user: `cargo build --release && ./packaging/install.sh`, then `strokebind list-devices`.
- [ ] You have 3-4 free hours after each post. Replies in the first hours decide how a post does; answer every comment.
- [ ] Order: Show HN first (a weekday, morning in US time is the usual advice), then r/linux and r/rust the same day, compositor subreddits one or two days later, AUR before or with the first post so the Arch install line works.
- [ ] Do not post the same text twice. Each channel below has its own wording.

## Show HN
Title: Show HN: strokebind – mouse-stroke gestures on any Linux Wayland compositor

URL: https://github.com/IvanKosk0v/strokebind

First comment (post it yourself right after submitting):

```
Hi HN. I wanted Easystroke-style mouse gestures on Wayland. Easystroke is discontinued, and the tools I found each work on one
compositor only (wlroots only, or Plasma only), so I wrote strokebind.

It reads your mouse through evdev, grabs it, and re-emits everything through a uinput virtual mouse. That is why it does not
need a compositor API: it works the same on Sway, Hyprland, KDE Plasma, GNOME and X11. Hold the trigger button (right by
default), draw a stroke like L or L-U, release, and it types a key combo or runs a command.

A few decisions that may be interesting:
- A press that does not become a stroke is replayed as a normal click, so right-click menus still work. You can also pick the
  side or middle button as the trigger.
- No ydotool/xdotool and no root daemon. Key combos go through its own virtual keyboard; a udev rule gives the seat user access.
- A grab on your only mouse is scary, so there are several ways out that never need the mouse: a button chord (left+right+middle
  by default), --timeout, and the kernel releases the grab when the process dies.
- `strokebind record` prints the stroke you just drew, `check-config` validates the TOML with line numbers, and bindings reload on save.
- Per-app bindings work on X11, Sway and Hyprland. On KDE and GNOME only global bindings work in 0.1.

I checked record and run by hand on bspwm (X11), Sway, Hyprland, KDE Plasma and GNOME. It does not remap keys and does not do
touchpad gestures. Feedback on the stroke recognizer and on other hardware is very welcome.
```

## r/linux
Title: strokebind: mouse-stroke gestures that work on every Wayland compositor (and X11)

Body:

```
Easystroke is gone, and the Wayland replacements I found only work on one compositor. strokebind is a small Rust daemon that
reads the mouse through evdev and forwards it through uinput, so it does not depend on a compositor API.

Hold the right button, draw a stroke, release: it presses a key combo or runs a command. A plain right-click still opens the
context menu. The demo GIF in the README shows browser back/forward and tab switching, then the escape chord.

- Sway, Hyprland, KDE Plasma, GNOME and X11, each checked by hand
- no root daemon: a udev rule plus a systemd user unit, installed per user with ./packaging/install.sh or from the .deb
- escape chord, --timeout, and the grab is released on crash or signal

Per-app bindings work on X11, Sway and Hyprland; KDE and GNOME get global bindings only for now.
Repo and .deb: https://github.com/IvanKosk0v/strokebind
```

## r/unixporn
Check the sidebar rules first. If the sub only takes rice screenshots, skip this one and tick the box anyway. If tool posts are allowed, use:

Title: [OTHER] strokebind: mouse gestures on Wayland, here on Sway

Body:

```
Right button + a stroke: back, forward, next tab. Clicks and menus still work. It reads the mouse through evdev and uses uinput,
so it is the same on Sway, Hyprland, KDE, GNOME and X11. Config is one small TOML file that reloads when saved.
https://github.com/IvanKosk0v/strokebind
```

Attach `docs/demo.gif`. Replace "here on Sway" with the desktop you record on, or drop it.

## r/rust
Title: strokebind: a mouse-gesture daemon in Rust (evdev grab + uinput)

Body:

```
I wrote a mouse-gesture daemon for Linux. The interesting parts for this sub:

- Pure recognizer: points -> 8-direction segments -> stroke string (`L-U`), with proptest properties (never panics, scale and
  jitter invariance).
- Device I/O is kept separate from logic, so the click/stroke state machine is tested against a fake event source. The real
  uinput tests are #[ignore]d.
- unsafe is confined to a sys module. A grab is released on drop, panic and SIGINT/SIGTERM (the process exits, the kernel drops
  the grab).
- Small dependency list (evdev, clap, serde/toml, notify, signal-hook, x11rb, serde_json). thiserror in the library, anyhow only in main.
- MSRV 1.85.1, measured with cargo-msrv; cargo-deny in CI.

Code and release: https://github.com/IvanKosk0v/strokebind
I would like feedback on the evdev/uinput handling and on the recognizer tolerance model.
```

## Compositor subreddits
Post on separate days, only where the sub allows tool posts. Keep each text short and compositor-specific.

### r/swaywm
Title: strokebind: mouse gestures for Sway, with per-app bindings

```
Hold a mouse button, draw a stroke, run a key combo or command. Per-app bindings use the focused window's app_id over SWAYSOCK
(or the XWayland class), with global bindings as the fallback. Plain clicks still work. Checked by hand on Sway.
https://github.com/IvanKosk0v/strokebind
```

### r/hyprland
Title: strokebind: mouse gestures for Hyprland, with per-app bindings

```
Mouse strokes that run key combos or commands, through evdev and uinput. Per-app bindings read the active window class from the
Hyprland socket at the moment you press the trigger. Checked by hand on Hyprland, including freeze_pointer.
https://github.com/IvanKosk0v/strokebind
```

### r/kde
Title: strokebind: mouse gestures on Plasma Wayland (global bindings)

```
Works on Plasma Wayland through evdev and uinput. In 0.1 it has global bindings only, no per-app bindings on KDE yet.
Right-click menus still work. Checked by hand on KDE Plasma.
https://github.com/IvanKosk0v/strokebind
```

### r/gnome
Title: strokebind: mouse gestures on GNOME Wayland (global bindings)

```
Works on GNOME Wayland through evdev and uinput. In 0.1 it has global bindings only, no per-app bindings on
GNOME yet. Checked by hand on GNOME.
https://github.com/IvanKosk0v/strokebind
```

## AUR
- [ ] Create the AUR package `strokebind` with the PKGBUILD below. It is a draft and has not been built: run `updpkgsums`, then `makepkg -si` in a clean chroot, and `namcap PKGBUILD` before submitting.
- [ ] Confirm `strokebind --version` prints 0.1.0 and `strokebind check-config /usr/share/doc/strokebind/config.example.toml` prints OK.
- [ ] Add the udev step to the first post's install line: after installing, log out and in once.

```
# Maintainer: Ivan Koskov
pkgname=strokebind
pkgver=0.1.0
pkgrel=1
pkgdesc='Mouse-stroke gestures for any Linux Wayland compositor and X11'
arch=('x86_64' 'aarch64')
url='https://github.com/IvanKosk0v/strokebind'
license=('MIT' 'Apache-2.0')
depends=('glibc' 'gcc-libs')
makedepends=('cargo')
optdepends=('libnotify: desktop notifications with feedback = "notify"')
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
  cd "$pkgname-$pkgver"
  cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
  cd "$pkgname-$pkgver"
  cargo build --release --frozen
}

package() {
  cd "$pkgname-$pkgver"
  install -Dm755 target/release/strokebind "$pkgdir/usr/bin/strokebind"
  install -Dm644 packaging/70-strokebind.rules "$pkgdir/usr/lib/udev/rules.d/70-strokebind.rules"
  install -Dm644 packaging/strokebind-deb.service "$pkgdir/usr/lib/systemd/user/strokebind.service"
  install -Dm644 packaging/config.example.toml "$pkgdir/usr/share/doc/strokebind/config.example.toml"
  install -Dm644 LICENSE-MIT "$pkgdir/usr/share/licenses/$pkgname/LICENSE-MIT"
  install -Dm644 LICENSE-APACHE "$pkgdir/usr/share/licenses/$pkgname/LICENSE-APACHE"
}
```

The systemd user unit is installed as in the `.deb`. On systems without systemd (Artix, Void) it is unused; start `strokebind run` from your session autostart instead.

## After posting
- [ ] Reply to every comment on each post the same day; thank people for bug reports and open an issue for each real one.
- [ ] Questions that come up twice go into the README FAQ.
- [ ] Do not argue about other tools. The honest limits are in the README: no key remapping, no touchpad gestures, no per-app bindings on KDE and GNOME in 0.1.
- [ ] Note what worked (stars per channel, issues opened) in STATE.md for the 0.2 planning.
