#!/usr/bin/env bash
# Print the absolute path of a working hkrestaurant binary.
set -euo pipefail

if [ -n "${HKRESTAURANT_BIN:-}" ] && [ -x "$HKRESTAURANT_BIN" ]; then
  printf '%s\n' "$HKRESTAURANT_BIN"
  exit 0
fi

if command -v hkrestaurant >/dev/null 2>&1; then
  command -v hkrestaurant
  exit 0
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -f "$ROOT/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
  (cd "$ROOT" && cargo build --release --locked >/dev/null)
  BIN="$ROOT/target/release/hkrestaurant"
  if [ -x "$BIN" ]; then
    printf '%s\n' "$BIN"
    exit 0
  fi
fi

echo "hkrestaurant: no binary found and no Cargo build available" >&2
echo "Set HKRESTAURANT_BIN, put hkrestaurant on PATH, or build with: cargo build --release --locked" >&2
exit 1
