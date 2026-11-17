#!/usr/bin/env bash
# Quiet gate: prints only "OK" on success, only the failing output otherwise (saves tokens).
set -u
cd "$(dirname "$0")/.."
out=$(cargo fmt --check 2>&1) || { echo "FMT FAILED (run cargo fmt):"; echo "$out" | head -40; exit 1; }
out=$(cargo clippy --all-targets --quiet -- -D warnings 2>&1) || { echo "CLIPPY FAILED:"; echo "$out" | head -60; exit 1; }
out=$(cargo test --quiet 2>&1) || { echo "TESTS FAILED:"; echo "$out" | tail -60; exit 1; }
if command -v cargo-deny >/dev/null 2>&1; then
  out=$(cargo deny check 2>&1) || { echo "DENY FAILED:"; echo "$out" | tail -40; exit 1; }
fi
echo OK
