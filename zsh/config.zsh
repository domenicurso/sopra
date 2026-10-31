if [[ -n ${_SOPRA_CONFIG_LOADED:-} ]]; then
    return 0
fi
typeset -g _SOPRA_CONFIG_LOADED=1

_sopra_config_path() {
    emulate -L zsh
    if [[ -n ${XDG_CONFIG_HOME:-} ]]; then
        print -r -- "$XDG_CONFIG_HOME/sopra/config.toml"
    else
        print -r -- "$HOME/.config/sopra/config.toml"
    fi
}

_sopra_load_prompt_config() {
    emulate -L zsh
    local config=$1 raw key encoded name
    [[ -f $config ]] || return 0
    raw=$("$SOPRA_BIN" --prompt-config "$config") || {
        print -u2 -- "sopra: could not load $config"
        return 1
    }
    while IFS=$'\t' read -r key encoded; do
        case $key in
            prompt) name=_SOPRA_CONFIG_PROMPT ;;
            rprompt) name=_SOPRA_CONFIG_RPROMPT ;;
            transient_prompt) name=_SOPRA_CONFIG_TRANSIENT_PROMPT ;;
            secondary_prompt) name=_SOPRA_CONFIG_SECONDARY_PROMPT ;;
            clock_format) name=_SOPRA_CONFIG_CLOCK_FORMAT ;;
            git_status_format) name=_SOPRA_CONFIG_GIT_STATUS_FORMAT ;;
            git_status_separator) name=_SOPRA_CONFIG_GIT_STATUS_SEPARATOR ;;
            git_status_item_format) name=_SOPRA_CONFIG_GIT_STATUS_ITEM_FORMAT ;;
            path_before) name=_SOPRA_CONFIG_PATH_BEFORE ;;
            path_selected) name=_SOPRA_CONFIG_PATH_SELECTED ;;
            path_after) name=_SOPRA_CONFIG_PATH_AFTER ;;
            cursor_animation) name=_SOPRA_CONFIG_CURSOR_ANIMATION ;;
            syntax_highlighting) name=_SOPRA_CONFIG_SYNTAX_HIGHLIGHTING ;;
            completion_enabled) name=_SOPRA_CONFIG_COMPLETION_ENABLED ;;
            autosuggestions) name=_SOPRA_CONFIG_AUTOSUGGESTIONS ;;
            autosuggestion_format) name=_SOPRA_CONFIG_AUTOSUGGESTION_FORMAT ;;
            autosuggestion_metadata_format) name=_SOPRA_CONFIG_AUTOSUGGESTION_METADATA_FORMAT ;;
            *) continue ;;
        esac
        _sopra_hex_decode "$encoded" || return 1
        typeset -g "$name=$_SOPRA_RESULT_BUFFER"
    done <<< "$raw"
}


sopra() {
    emulate -L zsh
    if [[ $# == 0 || ( $# == 1 && ${1-} == (--help|-h|help) ) ]]; then
        "$SOPRA_BIN" --help
        return $?
    fi
    print -u2 -- 'usage: sopra [--help]'
    return 2
}
