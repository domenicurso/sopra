#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
poc="${1:-all}"

case "$poc" in
  status)
    export KEEL_AUGMENT_POC=status
    export KEEL_AUGMENT_PROMPT_MODE=base
    ;;
  prompt)
    export KEEL_AUGMENT_POC=prompt
    export KEEL_AUGMENT_PROMPT_MODE=rust
    ;;
  cursor)
    export KEEL_AUGMENT_POC=cursor
    export KEEL_AUGMENT_CURSOR_MODE=highlight
    export KEEL_AUGMENT_CURSOR_STYLE=bar
    ;;
  syntax)
    export KEEL_AUGMENT_POC=syntax
    export KEEL_AUGMENT_CURSOR_MODE=highlight
    export KEEL_AUGMENT_CURSOR_STYLE=bar
    ;;
  widget)
    export KEEL_AUGMENT_POC=widget
    ;;
  dashboard|all)
    export KEEL_AUGMENT_POC=dashboard
    export KEEL_AUGMENT_PROMPT_MODE=rust
    export KEEL_AUGMENT_CURSOR_MODE=highlight
    export KEEL_AUGMENT_CURSOR_STYLE=bar
    ;;
  *)
    printf 'usage: %s [status|prompt|cursor|syntax|widget|dashboard|all]\n' "$0" >&2
    exit 2
    ;;
esac

exec "$repo_root/scripts/start-keel-augment.sh"
