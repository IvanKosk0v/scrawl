#!/usr/bin/env bash
# Per-user install of strokebind. Never runs as root and never calls sudo:
# the one root-only step (the udev rule) is printed for you to run.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
bin=${STROKEBIND_BIN:-$here/../target/release/strokebind}
cfg_home=${XDG_CONFIG_HOME:-$HOME/.config}
bindir=$HOME/.local/bin

if [ ! -f "$bin" ]; then
  echo "error: $bin not found; run 'cargo build --release' first" >&2
  exit 1
fi

install -D -m 755 "$bin" "$bindir/strokebind"
install -D -m 644 "$here/strokebind.service" "$cfg_home/systemd/user/strokebind.service"
if [ -e "$cfg_home/strokebind/config.toml" ]; then
  echo "kept existing $cfg_home/strokebind/config.toml"
else
  install -D -m 644 "$here/config.example.toml" "$cfg_home/strokebind/config.toml"
  echo "wrote starter config $cfg_home/strokebind/config.toml"
fi

if [ -z "${STROKEBIND_NO_SYSTEMCTL:-}" ] && command -v systemctl >/dev/null 2>&1; then
  systemctl --user daemon-reload
fi

cat <<MSG
Installed to $bindir/strokebind.

One step needs root (run it yourself): grant your seat user access to /dev/uinput and mice:
  cp $here/70-strokebind.rules /etc/udev/rules.d/ && udevadm control --reload && udevadm trigger
Then log out and in once, check the config, and start the daemon:
  strokebind check-config $cfg_home/strokebind/config.toml
  systemctl --user enable --now strokebind.service
Escape hatch if the mouse ever misbehaves: hold left+right+middle together, or: systemctl --user stop strokebind
MSG
