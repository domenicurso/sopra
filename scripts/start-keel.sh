#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
"$repo_root/scripts/build-keel.sh"

zsh_root=${KEEL_ZSH_PREFIX:-"$repo_root/target/keel-zsh"}
zshrc_root="$repo_root/target/keel-zshrc"
mkdir -p "$zshrc_root"

printf '%s\n' \
    "PROMPT='%F{green}%n%f in %F{cyan}%~%f %F{yellow}>%f '" \
    "RPROMPT=''" \
    "autoload -Uz compinit" \
    "compinit -C" \
    "source '$repo_root/zsh/keel.zsh'" \
    > "$zshrc_root/.zshrc"

export ZDOTDIR="$zshrc_root"
export KEEL_MODULE_PATH="$repo_root/target/debug"
exec "$zsh_root/bin/zsh" -di
