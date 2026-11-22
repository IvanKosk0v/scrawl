#!/usr/bin/env bash
# Per-user install of scrawl. Never runs as root and never calls sudo:
# the one root-only step (the udev rule) is printed for you to run.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
bin=${SCRAWL_BIN:-$here/../target/release/scrawl}
cfg_home=${XDG_CONFIG_HOME:-$HOME/.config}
bindir=$HOME/.local/bin

if [ ! -f "$bin" ]; then
  echo "error: $bin not found; run 'cargo build --release' first" >&2
  exit 1
fi

install -D -m 755 "$bin" "$bindir/scrawl"
install -D -m 644 "$here/scrawl.service" "$cfg_home/systemd/user/scrawl.service"
if [ -e "$cfg_home/scrawl/config.toml" ]; then
  echo "kept existing $cfg_home/scrawl/config.toml"
else
  install -D -m 644 "$here/config.example.toml" "$cfg_home/scrawl/config.toml"
  echo "wrote starter config $cfg_home/scrawl/config.toml"
fi

if [ -z "${SCRAWL_NO_SYSTEMCTL:-}" ] && command -v systemctl >/dev/null 2>&1; then
  systemctl --user daemon-reload
fi

cat <<MSG
Installed to $bindir/scrawl.

One step needs root (run it yourself): grant your seat user access to /dev/uinput and mice:
  cp $here/70-scrawl.rules /etc/udev/rules.d/ && udevadm control --reload && udevadm trigger
Then log out and in once, check the config, and start the daemon:
  scrawl check-config $cfg_home/scrawl/config.toml
  systemctl --user enable --now scrawl.service
Escape hatch if the mouse ever misbehaves: hold left+right+middle together, or: systemctl --user stop scrawl
MSG
