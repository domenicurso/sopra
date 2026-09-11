#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v expect >/dev/null 2>&1; then
  printf 'augment-pty: skipped (expect is not installed)\n'
  exit 0
fi

cargo build -p keel-augment --locked >/dev/null
exec expect "$repo_root/tests/augment-pty.exp"
