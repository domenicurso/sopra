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
        [[ $key =~ '^[a-z][a-z0-9_]*$' ]] || return 1
        name=_SOPRA_CONFIG_${(U)key}
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
