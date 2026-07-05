#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
zdotdir="$(mktemp -d "${TMPDIR:-/tmp}/keel-zdotdir.XXXXXX")"
module_artifact="$repo_root/target/zsh/keel.so"
shell_artifact="$repo_root/target/debug/keel-shell"

cleanup() {
  rm -rf "$zdotdir"
}

trap cleanup EXIT

if [[ "${KEEL_BUILD_ON_START:-0}" == "1" || ! -x "$shell_artifact" || ! -f "$module_artifact" ]]; then
  "$repo_root/scripts/build-zsh-module.sh" >/dev/null 2>&1
fi

cat >"$zdotdir/.zshrc" <<EOF
export KEEL_REPO_ROOT="$repo_root"
export KEEL_AUTO_START="\${KEEL_AUTO_START:-0}"
export KEEL_OWN_FRONTEND="\${KEEL_OWN_FRONTEND:-1}"
export KEEL_HIDE_ZSH_PROMPT="\${KEEL_HIDE_ZSH_PROMPT:-1}"
export KEEL_MINIMAL_SHELL="\${KEEL_MINIMAL_SHELL:-0}"

if [[ "\${KEEL_MINIMAL_SHELL:-0}" != "1" ]]; then
  [[ -r "\$HOME/.zprofile" ]] && source "\$HOME/.zprofile"
  [[ -r "\$HOME/.zshrc" ]] && source "\$HOME/.zshrc"
fi

source "$repo_root/shell/zsh/keel.zsh"
keel-session-loop
exit \$?
EOF

exec env ZDOTDIR="$zdotdir" zsh -i
