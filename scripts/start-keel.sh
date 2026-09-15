#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
"$repo_root/scripts/build-keel.sh"

export KEEL_REPO_ROOT="$repo_root"
export KEEL_BIN="$repo_root/target/debug/keel"
export KEEL_PROVIDER="$repo_root/zsh/keel-provider.zsh"
export KEEL_PROMPT='❯ '
export ZDOTDIR="$repo_root/examples/isolated"

exec zsh -di
