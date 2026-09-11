#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cargo test --workspace
cargo build -p keel-augment
zsh -n "$repo_root/zsh/keel-augment.zsh"

server_output="$({
  printf 'ping\n'
  printf 'render\techo\\nhi\\tthere\t2\t80\t24\tmain\t0\n'
  printf 'render\techo\\nhi\\tthere\t2\t80\t24\tmain\t0\n'
  printf 'stats\nshutdown\n'
} | "$repo_root/target/debug/keel-augment" --server)"
[[ "$server_output" == *pong* ]]
[[ "$server_output" == *"requests=2 rendered_frames=1 cache_hits=1"* ]]
[[ "$server_output" == *"cursor_directives=1"* ]]
[[ "$server_output" == *bye* ]]

KEEL_AUGMENT_ROOT="$repo_root" \
KEEL_AUGMENT_BINARY="$repo_root/target/debug/keel-augment" \
zsh -dfc '
  typeset -g BUFFER="echo hi"
  typeset -g CURSOR=5
  typeset -g COLUMNS=80
  typeset -g KEYMAP=main
  typeset -g ZLE_RPROMPT_INDENT=7
  typeset -ga region_highlight
  region_highlight=("0 1 fg=red")
  PROMPT="%~ %# "
  source "$KEEL_AUGMENT_ROOT/zsh/keel-augment.zsh"
  [[ $_KEEL_AUGMENT_ACTIVE -eq 0 ]]
  keel-augment-enable
  [[ $_KEEL_AUGMENT_ACTIVE -eq 1 ]]
  [[ "$ZLE_RPROMPT_INDENT" == 0 ]]
  _keel-augment-pre-redraw
  [[ "$RPROMPT" == *Keel* ]]
  keel-augment-disable
  [[ -z "$RPROMPT" ]]
  [[ "$ZLE_RPROMPT_INDENT" == 7 ]]
  [[ "${region_highlight[1]}" == "0 1 fg=red" ]]
' 

KEEL_AUGMENT_ROOT="$repo_root" \
KEEL_AUGMENT_BINARY="$repo_root/target/debug/keel-augment" \
KEEL_AUGMENT_CURSOR_MODE=highlight \
KEEL_AUGMENT_CURSOR_STYLE=bar \
zsh -dfc '
  typeset -g BUFFER="echo hi"
  typeset -g CURSOR=5
  typeset -g COLUMNS=80
  typeset -g KEYMAP=main
  typeset -ga region_highlight
  region_highlight=("0 1 fg=red")
  source "$KEEL_AUGMENT_ROOT/zsh/keel-augment.zsh"
  keel-augment-enable
  _keel-augment-pre-redraw
  [[ "$ZLE_RPROMPT_INDENT" == 0 ]]
  [[ "${region_highlight[1]}" == "0 1 fg=red" ]]
  [[ "${region_highlight[2]}" == *"4 5"* ]]
  [[ "$_KEEL_AUGMENT_CURSOR_STYLE" == bar ]]
  keel-augment-disable
  [[ "${region_highlight[1]}" == "0 1 fg=red" ]]
  [[ "$#region_highlight" == 1 ]]
'
