_keel_unbind_keys() {
    (( _KEEL_ZSH_KEYS )) || return 0
    local map
    for map in ${(k)_KEEL_SAVED_TAB_BINDINGS}; do
        bindkey -M "$map" '^I' "${_KEEL_SAVED_TAB_BINDINGS[$map]}"
    done
    for map in ${(k)_KEEL_SAVED_UP_BINDINGS}; do
        bindkey -M "$map" '^[[A' "${_KEEL_SAVED_UP_BINDINGS[$map]}"
    done
    for map in ${(k)_KEEL_SAVED_DOWN_BINDINGS}; do
        bindkey -M "$map" '^[[B' "${_KEEL_SAVED_DOWN_BINDINGS[$map]}"
    done
    for map in ${(k)_KEEL_SAVED_ESCAPE_BINDINGS}; do
        bindkey -M "$map" '^[' "${_KEEL_SAVED_ESCAPE_BINDINGS[$map]}"
    done
    for map in ${(k)_KEEL_SAVED_INTERRUPT_BINDINGS}; do
        bindkey -M "$map" '^C' "${_KEEL_SAVED_INTERRUPT_BINDINGS[$map]}"
    done
    for map in ${(k)_KEEL_SAVED_ENTER_BINDINGS}; do
        bindkey -M "$map" '^M' "${_KEEL_SAVED_ENTER_BINDINGS[$map]}"
    done
    for map in ${(k)_KEEL_SAVED_LINEFEED_BINDINGS}; do
        bindkey -M "$map" '^J' "${_KEEL_SAVED_LINEFEED_BINDINGS[$map]}"
    done
    _KEEL_SAVED_TAB_BINDINGS=()
    _KEEL_SAVED_UP_BINDINGS=()
    _KEEL_SAVED_DOWN_BINDINGS=()
    _KEEL_SAVED_ESCAPE_BINDINGS=()
    _KEEL_SAVED_INTERRUPT_BINDINGS=()
    _KEEL_SAVED_ENTER_BINDINGS=()
    _KEEL_SAVED_LINEFEED_BINDINGS=()
    _KEEL_ZSH_KEYS=0
}

_keel_disable() {
    if (( _KEEL_ZSH_HOOKS )); then
        _keel_completion_stop_pending
        add-zle-hook-widget -d line-pre-redraw _keel_completion_pre_redraw
        add-zle-hook-widget -d line-init _keel_completion_line_init
        add-zle-hook-widget -d line-finish _keel_completion_line_finish
        add-zle-hook-widget -d line-init keel-native-line-init
        add-zle-hook-widget -d line-finish keel-native-line-finish
        _keel_unbind_keys
        _KEEL_ZSH_HOOKS=0
    fi
    zmodload -u keel 2>/dev/null || true
}

_keel_status() {
    if (( _KEEL_ZSH_HOOKS )); then
        print 'keel: enabled'
    else
        print 'keel: disabled'
    fi
}

_keel_enable() {
    if (( _KEEL_ZSH_HOOKS )); then
        return 0
    fi
    if ! zmodload -i keel; then
        print -u2 'keel: native module unavailable'
        return 1
    fi
    add-zle-hook-widget line-init keel-native-line-init
    add-zle-hook-widget line-finish keel-native-line-finish
    add-zle-hook-widget line-pre-redraw _keel_completion_pre_redraw
    add-zle-hook-widget line-init _keel_completion_line_init
    add-zle-hook-widget line-finish _keel_completion_line_finish
    _keel_bind_keys
    _KEEL_ZSH_HOOKS=1
}

keel() {
    local subcommand=${1:-status}
    case "$subcommand" in
        status)
            (( $# <= 1 )) || { print -u2 'usage: keel status'; return 2; }
            _keel_status
            ;;
        disable)
            (( $# <= 1 )) || { print -u2 'usage: keel disable'; return 2; }
            _keel_disable
            ;;
        enable)
            (( $# <= 1 )) || { print -u2 'usage: keel enable'; return 2; }
            _keel_enable
            ;;
        help|-h|--help)
            print 'usage: keel {status|enable|disable|help}'
            ;;
        *)
            print -u2 "keel: unknown subcommand: $subcommand"
            print -u2 'usage: keel {status|enable|disable|help}'
            return 2
            ;;
    esac
}
