#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$repo_root"
./scripts/build-zsh-module.sh >/dev/null
zsh -n "$repo_root/shell/zsh/keel.zsh"
zsh -fc "zmodload zsh/zle; module_path=('$repo_root/target/zsh' \$module_path); zmodload keel; keel-activate; zle -l keel-accept-line >/dev/null; zle -l keel-edit-line >/dev/null; keel-deactivate"
zsh -fc "zmodload zsh/zle; KEEL_REPO_ROOT='$repo_root'; KEEL_SUPPRESS_LIMITATION_WARNING=1; source '$repo_root/shell/zsh/keel.zsh'; [[ -z \$PROMPT ]]; [[ -z \$RPROMPT ]]; zle -l keel-accept-line >/dev/null; zle -l keel-edit-line >/dev/null; zle -l zle-line-init >/dev/null; bindkey '^M' | grep -q 'keel-accept-line'"

printf 'module-backed phase-1/2 artifacts look loadable\n'
