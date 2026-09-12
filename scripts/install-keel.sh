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
    yellow=$'\033[33m'
    reset=$'\033[0m'
else
    bold=''
    cyan=''
    dim=''
    green=''
    yellow=''
    reset=''
fi

animate=0
if [[ -t 1 && ${TERM:-dumb} != dumb ]]; then
    animate=1
fi

build_log=$(mktemp "${TMPDIR:-/tmp}/keel-install.XXXXXX")
cleanup() {
    rm -f "$build_log"
}
trap cleanup EXIT

loader_frames=('-' '\' '|' '/')
run_step() {
    local label=$1
    shift
    local status=0

    if (( animate )); then
        "$@" >"$build_log" 2>&1 &
        local pid=$!
        local frame_index=0
        while kill -0 "$pid" 2>/dev/null; do
            local frame=${loader_frames[$((frame_index % ${#loader_frames[@]}))]}
            printf '\r\033[K%b%s%b %s' "$cyan" "$label" "$reset" "$frame"
            sleep 0.1
            frame_index=$((frame_index + 1))
        done
        wait "$pid" || status=$?
    else
        printf '%s\n' "$label"
        "$@" >"$build_log" 2>&1 || status=$?
    fi

    if (( status != 0 )); then
        printf '%b\n' "${bold}${cyan}${label} failed${reset}" >&2
        cat "$build_log" >&2
        return "$status"
    fi

    if (( animate )); then
        printf '\r\033[K%b%s%b %b%s%b\n' "$cyan" "$label" "$reset" "$green" 'done' "$reset"
    fi
}

run_step 'Building Keel' env KEEL_ZSH_PREFIX="$zsh_prefix" "$repo_root/scripts/build-keel.sh"

install_files() {
    mkdir -p "$install_prefix/bin" "$install_prefix/share" "$module_dir" "$startup_dir"
    install -m 0755 "$repo_root/target/debug/keel.so" "$module_dir/keel.so"
    install -m 0644 "$repo_root/zsh/keel.zsh" "$install_prefix/share/keel.zsh"
    printf 'keel-install-v1\n' > "$install_prefix/.keel-install"

    launcher="$install_prefix/bin/keel"
    quoted_prefix=$(printf '%q' "$install_prefix")
    quoted_startup_dir=$(printf '%q' "$startup_dir")
    printf '%s\n' \
        '#!/usr/bin/env bash' \
        'set -euo pipefail' \
        "prefix=$quoted_prefix" \
        'export KEEL_MODULE_PATH="${KEEL_MODULE_PATH:-$prefix/zsh/lib/zsh/5.9/zsh}"' \
        'export KEEL_PRIVATE_SHELL=1' \
        "export ZDOTDIR=\${KEEL_ZDOTDIR:-$quoted_startup_dir}" \
        'case ${1:-} in' \
        '    status)' \
        '        printf "keel: installed at %s; current shell is not managed\\n" "$prefix"' \
        '        exit 0' \
        '        ;;' \
        '    disable|enable)' \
        '        printf "keel: %s must be run inside the active Keel shell\\n" "$1" >&2' \
        '        exit 2' \
        '        ;;' \
        '    help|-h|--help)' \
        '        printf "usage: keel [shell-options]\\n       keel status\\n       keel enable|disable\\n"' \
        '        exit 0' \
        '        ;;' \
        'esac' \
        'if [[ $# -eq 0 ]]; then set -- -il; fi' \
        'exec "$prefix/zsh/bin/zsh" "$@"' \
        > "$launcher"
    chmod 0755 "$launcher"

    printf '%s\n' \
        '# Keel keeps this startup directory separate from the user Zsh config.' \
        'if [[ -r "$HOME/.zshenv" ]]; then' \
        '    _keel_zdotdir=$ZDOTDIR' \
        '    source "$HOME/.zshenv"' \
        '    export ZDOTDIR=$_keel_zdotdir' \
        '    unset _keel_zdotdir' \
        'fi' \
        > "$startup_dir/.zshenv"

    printf '%s\n' \
        '# Login environment is safe to reuse; interactive plugin setup is opt-in.' \
        'if [[ -r "$HOME/.zprofile" ]]; then source "$HOME/.zprofile"; fi' \
        > "$startup_dir/.zprofile"

    printf '%s\n' \
        '# Import the user rc only when explicitly requested; it may run network/plugin setup.' \
        'if [[ ${KEEL_SOURCE_USER_RC:-0} == 1 && -r "$HOME/.zshrc" ]]; then' \
        '    source "$HOME/.zshrc"' \
        'fi' \
        'if [[ ${KEEL_SOURCE_USER_RC:-0} != 1 ]]; then' \
        "    PROMPT='%F{green}%n%f in %F{cyan}%~%f %F{yellow}>%f '" \
        "    RPROMPT=''" \
        'fi' \
        'autoload -Uz compinit' \
        'compinit -C' \
        "source $quoted_prefix/share/keel.zsh" \
        > "$startup_dir/.zshrc"

    printf '%s\n' \
        'if [[ -r "$HOME/.zlogin" ]]; then source "$HOME/.zlogin"; fi' \
        > "$startup_dir/.zlogin"
}

run_step 'Installing Keel' install_files

launch_command=$(printf '%q' "$install_prefix/bin/keel")
printf '%b\n' "${bold}${cyan}Keel installed${reset}"
printf '  %bprefix:%b %b%s%b\n' "$dim" "$reset" "$yellow" "$install_prefix" "$reset"
printf '  %blaunch:%b %b%s -il%b\n' "$dim" "$reset" "$green" "$launch_command" "$reset"
printf '  %bcommands:%b %bkeel status | keel enable | keel disable%b\n' "$dim" "$reset" "$green" "$reset"
