#!/usr/bin/env bash
# Print the absolute path of a working hkrestaurant binary, or exit 1.
set -euo pipefail

if [[ -n "${HKRESTAURANT_BIN:-}" && -x "${HKRESTAURANT_BIN}" ]]; then
  printf '%s\n' "$(cd "$(dirname "${HKRESTAURANT_BIN}")" && pwd)/$(basename "${HKRESTAURANT_BIN}")"
  exit 0
fi

if resolved="$(command -v hkrestaurant 2>/dev/null)" && [[ -n "$resolved" ]]; then
  printf '%s\n' "$resolved"
  exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(dirname "$script_dir")"

if [[ -f "$script_dir/install.sh" ]] &&
  path="$(sh "$script_dir/install.sh")" &&
  [[ -n "$path" && -x "$path" ]]; then
  printf '%s\n' "$path"
  exit 0
fi

if [[ -f "$repo_root/Cargo.toml" ]] && command -v cargo >/dev/null 2>&1; then
  if (cd "$repo_root" && cargo build --release --locked >&2); then
    path="$repo_root/target/release/hkrestaurant"
    if [[ -x "$path" ]]; then
      printf '%s\n' "$path"
      exit 0
    fi
  fi
fi

printf 'ensure-binary: no hkrestaurant binary via HKRESTAURANT_BIN, PATH, install.sh or cargo build\n' >&2
exit 1
