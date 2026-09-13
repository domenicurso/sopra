_keel_original_binding() {
    local map=${KEYMAP:-main}
    case "$1" in
        tab) print -r -- "${_KEEL_SAVED_TAB_BINDINGS[$map]:-undefined-key}" ;;
        up) print -r -- "${_KEEL_SAVED_UP_BINDINGS[$map]:-undefined-key}" ;;
        down) print -r -- "${_KEEL_SAVED_DOWN_BINDINGS[$map]:-undefined-key}" ;;
        escape) print -r -- "${_KEEL_SAVED_ESCAPE_BINDINGS[$map]:-undefined-key}" ;;
        enter) print -r -- "${_KEEL_SAVED_ENTER_BINDINGS[$map]:-accept-line}" ;;
        linefeed) print -r -- "${_KEEL_SAVED_LINEFEED_BINDINGS[$map]:-accept-line}" ;;
        interrupt) print -r -- "${_KEEL_SAVED_INTERRUPT_BINDINGS[$map]:-undefined-key}" ;;
    esac
}

_keel_interrupt_widget() {
    _keel_abort_line
    return 0
}

_keel_abort_line() {
    (( _KEEL_ABORTING )) && return 0
    _KEEL_ABORTING=1
    _keel_cursor_stop
    _keel_completion_stop_pending
    zle keel-native-line-finish 2>/dev/null || true
    zle send-break
    _KEEL_ABORTING=0
}

_keel_accept_widget() {
    local fallback=$(_keel_original_binding tab)
    zle keel-native-accept-selection
    local result=$?
    if (( result != 0 )); then
        zle "$fallback"
    fi
    return 0
}

_keel_select_previous_widget() {
    local fallback=$(_keel_original_binding up)
    zle keel-native-select-previous
    local result=$?
    if (( result != 0 )); then
        zle "$fallback"
    fi
    return 0
}

_keel_select_next_widget() {
    local fallback=$(_keel_original_binding down)
    zle keel-native-select-next
    local result=$?
    if (( result != 0 )); then
        zle "$fallback"
    fi
    return 0
}

_keel_dismiss_widget() {
    local fallback=$(_keel_original_binding escape)
    zle keel-native-dismiss-overlay
    local result=$?
    if (( result != 0 )); then
        zle "$fallback"
    fi
    return 0
}

_keel_accept_enter_widget() {
    if [[ -z $BUFFER ]]; then
        zle redisplay
        return 0
    fi
    local fallback=$(_keel_original_binding enter)
    zle "$fallback"
    return 0
}

_keel_accept_linefeed_widget() {
    if [[ -z $BUFFER ]]; then
        zle redisplay
        return 0
    fi
    local fallback=$(_keel_original_binding linefeed)
    zle "$fallback"
    return 0
}

_keel_bind_key() {
    local map=$1
    local key=$2
    local widget=$3
    local bindings=$4
    local binding original

    if ! bindkey -lL "$map" >/dev/null 2>&1; then
        return 0
    fi
    binding=$(bindkey -M "$map" "$key" 2>/dev/null) || return 0
    original=${binding##* }
    [[ -n $original ]] || original=undefined-key
    case "$bindings" in
        _KEEL_SAVED_TAB_BINDINGS) _KEEL_SAVED_TAB_BINDINGS[$map]=$original ;;
        _KEEL_SAVED_UP_BINDINGS) _KEEL_SAVED_UP_BINDINGS[$map]=$original ;;
        _KEEL_SAVED_DOWN_BINDINGS) _KEEL_SAVED_DOWN_BINDINGS[$map]=$original ;;
        _KEEL_SAVED_ESCAPE_BINDINGS) _KEEL_SAVED_ESCAPE_BINDINGS[$map]=$original ;;
        _KEEL_SAVED_ENTER_BINDINGS) _KEEL_SAVED_ENTER_BINDINGS[$map]=$original ;;
        _KEEL_SAVED_LINEFEED_BINDINGS) _KEEL_SAVED_LINEFEED_BINDINGS[$map]=$original ;;
        _KEEL_SAVED_INTERRUPT_BINDINGS) _KEEL_SAVED_INTERRUPT_BINDINGS[$map]=$original ;;
    esac
    bindkey -M "$map" "$key" "$widget"
}

_keel_bind_keys() {
    (( _KEEL_ZSH_KEYS )) && return 0
    local map
    for map in main emacs viins; do
        _keel_bind_key "$map" '^I' _keel_accept_widget _KEEL_SAVED_TAB_BINDINGS
        _keel_bind_key "$map" '^[[A' _keel_select_previous_widget _KEEL_SAVED_UP_BINDINGS
        _keel_bind_key "$map" '^[[B' _keel_select_next_widget _KEEL_SAVED_DOWN_BINDINGS
        _keel_bind_key "$map" '^[' _keel_dismiss_widget _KEEL_SAVED_ESCAPE_BINDINGS
        _keel_bind_key "$map" '^C' _keel_interrupt_widget _KEEL_SAVED_INTERRUPT_BINDINGS
        _keel_bind_key "$map" '^M' _keel_accept_enter_widget _KEEL_SAVED_ENTER_BINDINGS
        _keel_bind_key "$map" '^J' _keel_accept_linefeed_widget _KEEL_SAVED_LINEFEED_BINDINGS
    done
    _KEEL_ZSH_KEYS=1
}
