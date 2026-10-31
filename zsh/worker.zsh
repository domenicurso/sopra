_sopra_edit() {
    emulate -L zsh
    local raw
    if _sopra_host_active; then
        printf '\e]777;sopra;editing;%s\a' "${CONTEXT:-start}"
    fi
    local -x SOPRA_BUILTINS SOPRA_ALIASES SOPRA_FUNCTIONS SOPRA_COMMANDS SOPRA_VARIABLES SOPRA_ARRAYS
    _sopra_shell_context
    local -a editor_args=(--buffer "$BUFFER" --cursor "$CURSOR" --cwd "$PWD")
    [[ $CONTEXT == cont ]] && editor_args+=(--secondary)
    raw=$(
        SOPRA_WIDGET_STATUS="$_SOPRA_WIDGET_STATUS" \
        SOPRA_WIDGET_DURATION="$_SOPRA_WIDGET_DURATION" \
        SOPRA_WIDGET_USER="$USER" SOPRA_WIDGET_HOST="$HOST" \
        _sopra_worker_session "${editor_args[@]}" --history-file =(_sopra_history_snapshot) </dev/tty
    ) || return 1
    _sopra_parse_result "$raw"
}

_sopra_history_snapshot() {
    zmodload zsh/parameter || return 1
    local -a events=(${(Onk)history})
    local oldest=${events[-1]:-0}
    printf 'H3\0%s\0' "${HISTFILE:-}"
    if [[ -r ${HISTFILE:-} ]]; then
        # Persisted timestamps are cheaper to read in Rust than to reformat in
        # fc for every prompt. Only the newest event may not be saved yet.
        oldest=${events[1]:-0}
    fi
    if (( ${#events} )); then
        fc -l -t '%s' "$oldest" "$events[1]"
    fi
    printf '\0'
    local event
    for event in $events; do
        printf '%s\0%s\0' "$event" "${history[$event]}"
    done
}
