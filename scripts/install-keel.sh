#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
install_prefix=${KEEL_INSTALL_PREFIX:-"$HOME/.local/keel"}
zsh_prefix="$install_prefix/zsh"
module_dir="$zsh_prefix/lib/zsh/5.9/zsh"
startup_dir="$install_prefix/etc/zsh"

if [[ -t 1 && -z ${NO_COLOR:-} ]]; then
    bold=$'\033[1m'
    cyan=$'\033[36m'
    dim=$'\033[2m'
    green=$'\033[32m'
    red=$'\033[31m'
    reset=$'\033[0m'
else
    bold=''
    cyan=''
    dim=''
    green=''
    red=''
    reset=''
fi

animate=0
if [[ -t 1 && ${TERM:-dumb} != dumb ]]; then
    animate=1
fi

build_log=$(mktemp "${TMPDIR:-/tmp}/keel-install.XXXXXX")
step_dir=''
cleanup() {
    rm -f "$build_log"
    if [[ -n $step_dir ]]; then
        rm -rf -- "$step_dir"
    fi
}
trap cleanup EXIT

source "$repo_root/scripts/install/output.sh"
source "$repo_root/scripts/install/files.sh"

run_step 'Building Keel' env KEEL_PROGRESS=1 KEEL_ZSH_PREFIX="$zsh_prefix" "$repo_root/scripts/build-keel.sh"

run_step 'Installing Keel' install_files

launch_command=$(printf '%q' "$install_prefix/bin/keel")
printf '%b\n' "${bold}${cyan}Keel installed${reset}"
printf 'Launch Keel with %b%s -il%b.\n' "$green" "$launch_command" "$reset"
printf 'After it starts, run %bkeel status%b to confirm the shell is enabled.\n' "$green" "$reset"
