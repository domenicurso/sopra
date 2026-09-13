install_files() {
    progress() {
        printf 'keel-progress:%s\n' "$*"
    }

    progress 'Creating installation directories'
    mkdir -p "$install_prefix/bin" "$install_prefix/share" "$module_dir" "$startup_dir"
    progress 'Installing native module'
    install -m 0755 "$repo_root/target/debug/keel.so" "$module_dir/keel.so"
    local shell_files=("$repo_root"/zsh/*.zsh "$repo_root"/zsh/completion/*.zsh)
    local shell_count=${#shell_files[@]}
    local shell_index=0
    for shell_file in "${shell_files[@]}"; do
        shell_index=$((shell_index + 1))
        progress "Installing shell loader ($shell_index/$shell_count): $(basename "$shell_file")"
        local relative=${shell_file#"$repo_root/zsh/"}
        local destination="$install_prefix/share/$relative"
        mkdir -p "$(dirname "$destination")"
        install -m 0644 "$shell_file" "$destination"
    done
    progress 'Writing installation marker'
    printf '%s\n' 'keel-install-v1' > "$install_prefix/.keel-install"

    local launcher="$install_prefix/bin/keel"
    progress 'Writing Keel launcher'
    local quoted_prefix
    local quoted_startup_dir
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
