if [[ -n ${_SOPRA_SETTINGS_LOADED:-} ]]; then
    return 0
fi
typeset -g _SOPRA_SETTINGS_LOADED=1

_sopra_config_path() {
    emulate -L zsh
    if [[ -n ${XDG_CONFIG_HOME:-} ]]; then
        print -r -- "$XDG_CONFIG_HOME/sopra/settings"
    else
        print -r -- "$HOME/.config/sopra/settings"
    fi
}

_sopra_load_settings() {
    emulate -L zsh
    local force=${1:-0} config key value
    config=$(_sopra_config_path)
    [[ -r $config ]] || return 0
    while IFS='=' read -r key value; do
        case $key in
            prompt)
                if (( force )) || [[ -z ${SOPRA_PROMPT+x} ]]; then
                    typeset -g SOPRA_PROMPT=$value
                fi
                ;;
            rprompt)
                if (( force )) || [[ -z ${SOPRA_RPROMPT+x} ]]; then
                    typeset -g SOPRA_RPROMPT=$value
                fi
                ;;
            transient_prompt)
                if (( force )) || [[ -z ${SOPRA_TRANSIENT_PROMPT+x} ]]; then
                    typeset -g SOPRA_TRANSIENT_PROMPT=$value
                fi
                ;;
            clock_format)
                if (( force )) || [[ -z ${SOPRA_CLOCK_FORMAT+x} ]]; then
                    typeset -g SOPRA_CLOCK_FORMAT=$value
                fi
                ;;
            cursor_animation)
                if (( force )) || [[ -z ${SOPRA_CURSOR_ANIMATION+x} ]]; then
                    typeset -g SOPRA_CURSOR_ANIMATION=$value
                fi
                ;;
            syntax_highlighting)
                if (( force )) || [[ -z ${SOPRA_SYNTAX_HIGHLIGHTING+x} ]]; then
                    typeset -g SOPRA_SYNTAX_HIGHLIGHTING=$value
                fi
                ;;
            completion_enabled)
                if (( force )) || [[ -z ${SOPRA_COMPLETION_ENABLED+x} ]]; then
                    typeset -g SOPRA_COMPLETION_ENABLED=$value
                fi
                ;;
        esac
    done < "$config"
}

_sopra_settings() {
    emulate -L zsh
    SOPRA_PROMPT="$SOPRA_PROMPT" \
        SOPRA_RPROMPT="$SOPRA_RPROMPT" \
        SOPRA_TRANSIENT_PROMPT="$SOPRA_TRANSIENT_PROMPT" \
        SOPRA_CLOCK_FORMAT="$SOPRA_CLOCK_FORMAT" \
        SOPRA_CURSOR_ANIMATION="${SOPRA_CURSOR_ANIMATION:-1}" \
        SOPRA_SYNTAX_HIGHLIGHTING="${SOPRA_SYNTAX_HIGHLIGHTING:-1}" \
        SOPRA_COMPLETION_ENABLED="${SOPRA_COMPLETION_ENABLED:-1}" \
        "$SOPRA_BIN" --settings </dev/tty
    local exit_code=$?
    (( exit_code == 0 )) && _sopra_load_settings 1
    return $exit_code
}

sopra() {
    emulate -L zsh
    if [[ $# == 1 && ${1-} == (--help|-h|help) ]]; then
        "$SOPRA_BIN" --help
        return $?
    fi
    if [[ ${1-} == settings ]]; then
        if [[ $# == 1 ]]; then
            _sopra_settings
            return $?
        fi
        if [[ $# == 2 && ${2-} == (--help|-h|help) ]]; then
            "$SOPRA_BIN" settings --help
            return $?
        fi
    fi
    print -u2 -- 'usage: sopra settings'
    return 2
}
