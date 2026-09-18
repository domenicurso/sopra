#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
"$repo_root/scripts/build.sh"

export SOPRA_REPO_ROOT="$repo_root"
export SOPRA_BIN="$repo_root/target/debug/sopra"
export SOPRA_PROMPT='%n in %~ $ '
export ZDOTDIR="$repo_root/examples/isolated"

exec zsh -di
