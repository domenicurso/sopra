#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cargo build -p keel-augment

if [[ "${TERM:-dumb}" == "dumb" ]]; then
  export TERM=xterm-256color
fi

exec env \
  KEEL_AUGMENT_ROOT="$repo_root" \
  KEEL_AUGMENT_BINARY="$repo_root/target/debug/keel-augment" \
  ZDOTDIR="$repo_root/tests/fixtures/augment-shell" \
  zsh -di
