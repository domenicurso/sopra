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
    red=$'\033[31m'
    reset=$'\033[0m'
else
    bold=''
    cyan=''
    dim=''
    green=''
    yellow=''
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

loader_frames=('-' '\' '|' '/')
render_step() {
    local label=$1
    local subtext=$2
    local frame=$3

    if (( animate )); then
        if [[ -n $subtext ]]; then
            printf '\r\033[K%b%s%b %b%s%b %s' \
                "$cyan" "$label" "$reset" "$dim" "$subtext" "$reset" "$frame"
        else
            printf '\r\033[K%b%s%b %s' "$cyan" "$label" "$reset" "$frame"
        fi
    elif [[ -n $subtext ]]; then
        printf '%s: %s\n' "$label" "$subtext"
    else
        printf '%s\n' "$label"
    fi
}

run_step() {
    local label=$1
    shift
    local status=0
    local subtext='starting'
    local frame_index=0
    local line

    step_dir=$(mktemp -d "${TMPDIR:-/tmp}/keel-step.XXXXXX")
    local step_pipe="$step_dir/output"
    mkfifo "$step_pipe"
    : > "$build_log"

    "$@" >"$step_pipe" 2>&1 &
    local pid=$!
    while IFS= read -r line || [[ -n $line ]]; do
        printf '%s\n' "$line" >> "$build_log"
        case $line in
            keel-progress:*)
                subtext=${line#keel-progress:}
                ;;
            patching\ file\ *)
                subtext=${line#patching file }
                subtext=${subtext#\'}
                subtext=${subtext%\'}
                subtext="Patching $subtext"
                ;;
            Patching\ file\ *)
                subtext=${line#Patching file }
                subtext=${subtext#\'}
                subtext=${subtext%\'}
                subtext="Patching $subtext"
                ;;
            *)
                continue
                ;;
        esac

        local frame=${loader_frames[$((frame_index % ${#loader_frames[@]}))]}
        render_step "$label" "$subtext" "$frame"
        frame_index=$((frame_index + 1))
    done < "$step_pipe"
    wait "$pid" || status=$?
    rm -rf -- "$step_dir"
    step_dir=''

    if (( status != 0 )); then
        if (( animate )); then
            printf '\r\033[K%b%s%b\n' "${bold}${red}" "$label failed" "$reset" >&2
        else
            printf '%s failed\n' "$label" >&2
        fi
        cat "$build_log" >&2
        return "$status"
    fi

    if (( animate )); then
        printf '\r\033[K%b%s%b %b%s%b\n' "$cyan" "$label" "$reset" "$green" 'done' "$reset"
    fi
}

run_step 'Building Keel' env KEEL_PROGRESS=1 KEEL_ZSH_PREFIX="$zsh_prefix" "$repo_root/scripts/build-keel.sh"

install_files() {
    progress() {
        printf 'keel-progress:%s\n' "$*"
    }

    progress 'Creating installation directories'
    mkdir -p "$install_prefix/bin" "$install_prefix/share" "$module_dir" "$startup_dir"
    progress 'Installing native module'
    install -m 0755 "$repo_root/target/debug/keel.so" "$module_dir/keel.so"
    local shell_files=("$repo_root"/zsh/*.zsh)
    local shell_count=${#shell_files[@]}
    local shell_index=0
    for shell_file in "$repo_root"/zsh/*.zsh; do
        shell_index=$((shell_index + 1))
        progress "Installing shell loader ($shell_index/$shell_count): $(basename "$shell_file")"
        install -m 0644 "$shell_file" "$install_prefix/share/$(basename "$shell_file")"
    done
    progress 'Writing installation marker'
    printf 'keel-install-v1\n' > "$install_prefix/.keel-install"

    launcher="$install_prefix/bin/keel"
    progress 'Writing Keel launcher'
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
        '# Reuse the user rc by default so functions, completions, aliases, and prompt stay intact.' \
        'if [[ ${KEEL_SOURCE_USER_RC:-1} == 1 && -r "$HOME/.zshrc" ]]; then' \
        '    source "$HOME/.zshrc"' \
        'fi' \
        'if [[ ${KEEL_SOURCE_USER_RC:-1} != 1 ]]; then' \
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
    progress 'Writing Zsh startup files'
}

run_step 'Installing Keel' install_files

launch_command=$(printf '%q' "$install_prefix/bin/keel")
printf '%b\n' "${bold}${cyan}Keel installed${reset}"
printf '  %bprefix:%b %b%s%b\n' "$dim" "$reset" "$yellow" "$install_prefix" "$reset"
printf '  %blaunch:%b %b%s -il%b\n' "$dim" "$reset" "$green" "$launch_command" "$reset"
printf '  %bisolated:%b %bKEEL_SOURCE_USER_RC=0 %s -il%b\n' "$dim" "$reset" "$green" "$launch_command" "$reset"
printf '  %bcommands:%b %bkeel status | keel enable | keel disable%b\n' "$dim" "$reset" "$green" "$reset"
