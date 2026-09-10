#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cargo test --workspace
cargo build -p keel-augment
zsh -n "$repo_root/zsh/keel-augment.zsh"

KEEL_AUGMENT_ROOT="$repo_root" \
KEEL_AUGMENT_BINARY="$repo_root/target/debug/keel-augment" \
zsh -dfc '
  typeset -g BUFFER="echo hi"
  typeset -g CURSOR=5
  typeset -g COLUMNS=80
  typeset -g KEYMAP=main
  PROMPT="%~ %# "
  source "$KEEL_AUGMENT_ROOT/zsh/keel-augment.zsh"
  [[ $_KEEL_AUGMENT_ACTIVE -eq 0 ]]
  keel-augment-enable
  [[ $_KEEL_AUGMENT_ACTIVE -eq 1 ]]
  _keel-augment-pre-redraw
  [[ "$RPROMPT" == *Keel* ]]
  keel-augment-disable
  [[ -z "$RPROMPT" ]]
' 
