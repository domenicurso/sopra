#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
"$repo_root/scripts/build-keel.sh"

zsh_root=${KEEL_ZSH_PREFIX:-"$repo_root/target/keel-zsh"}
zshrc_root=$(mktemp -d "$repo_root/target/keel-zshrc.XXXXXX")
trap 'rm -rf "$zshrc_root"' EXIT

printf '%s\n' \
    "PROMPT='%F{green}%n%f in %F{cyan}%~%f %F{yellow}>%f '" \
    "RPROMPT=''" \
    "source '$repo_root/zsh/keel.zsh'" \
    > "$zshrc_root/.zshrc"

ZDOTDIR="$zshrc_root" \
    KEEL_MODULE_PATH="$repo_root/target/debug" \
    "$zsh_root/bin/zsh" -di
