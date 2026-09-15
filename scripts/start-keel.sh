#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
"$repo_root/scripts/build-keel.sh"

export KEEL_REPO_ROOT="$repo_root"
export KEEL_BIN="$repo_root/target/debug/keel-demo"
export KEEL_PROMPT='keel-demo ❯ '
export ZDOTDIR="$repo_root/demo"

exec zsh -di
