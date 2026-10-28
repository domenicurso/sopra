_sopra_edit() {
    emulate -L zsh
    local raw expanded_prompt expanded_rprompt expanded_transient
    local -i arm_completion=${1:-0}
    local -i suppress_completion=${2:-0}
    local -x SOPRA_ALIASES SOPRA_FUNCTIONS SOPRA_COMMANDS SOPRA_VARIABLES SOPRA_ARRAYS
    _sopra_shell_context
    expanded_prompt=$(print -P -- "$PROMPT")
    expanded_rprompt=$(print -P -- "$RPROMPT")
    expanded_transient=$(print -P -- "${_sopra_transient_prompt-$_SOPRA_CONFIG_TRANSIENT_PROMPT}")
    local -a editor_args
    _sopra_editor_args "$arm_completion" "$suppress_completion"
    raw=$(
        SOPRA_PATH_BEFORE="$_SOPRA_CONFIG_PATH_BEFORE" \
        SOPRA_PATH_SELECTED="$_SOPRA_CONFIG_PATH_SELECTED" \
        SOPRA_PATH_AFTER="$_SOPRA_CONFIG_PATH_AFTER" \
        SOPRA_GIT_STATUS_ITEM_FORMAT="$_SOPRA_CONFIG_GIT_STATUS_ITEM_FORMAT" \
        SOPRA_GIT_STATUS_FORMAT="$_SOPRA_CONFIG_GIT_STATUS_FORMAT" \
        SOPRA_GIT_STATUS_SEPARATOR="$_SOPRA_CONFIG_GIT_STATUS_SEPARATOR" \
        SOPRA_WIDGET_STATUS="$_SOPRA_WIDGET_STATUS" \
        SOPRA_WIDGET_DURATION="$_SOPRA_WIDGET_DURATION" \
        SOPRA_WIDGET_USER="$USER" SOPRA_WIDGET_HOST="$HOST" \
        SOPRA_CLOCK_FORMAT="$_SOPRA_CONFIG_CLOCK_FORMAT" \
        SOPRA_EXPANDED_TRANSIENT_PROMPT="$expanded_transient" \
        SOPRA_EXPANDED_SECONDARY_PROMPT="$(print -P -- "$_SOPRA_CONFIG_SECONDARY_PROMPT")" \
        "$SOPRA_BIN" "${editor_args[@]}" </dev/tty
    ) || return 1
    _sopra_parse_result "$raw"
}

_sopra_editor_args() {
    editor_args=(
        --buffer "$BUFFER" \
        --cursor "$CURSOR" \
        --prompt "$expanded_prompt" \
        --rprompt "$expanded_rprompt" \
        --cwd "$PWD"
    )
    if (( ${1:-0} )); then
        editor_args+=(--arm-completion)
    fi
    if (( ${2:-0} )); then
        editor_args+=(--suppress-completion)
    fi
    if [[ ${(L)_SOPRA_CONFIG_COMPLETION_ENABLED} == (0|false|off|no) ]]; then
        editor_args+=(--disable-completion)
    fi
    if [[ ${(L)_SOPRA_CONFIG_SYNTAX_HIGHLIGHTING} == (0|false|off|no) ]]; then
        editor_args+=(--disable-syntax-highlighting)
    fi
    if [[ ${(L)_SOPRA_CONFIG_CURSOR_ANIMATION} == (0|false|off|no) ]]; then
        editor_args+=(--disable-cursor-animation)
    fi
}
