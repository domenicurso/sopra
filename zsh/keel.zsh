if [[ -n ${_KEEL_ZSH_LOADED:-} ]]; then
    return 0
fi

typeset -g _KEEL_ZSH_LOADED=1
typeset -g _KEEL_ZSH_HOOKS=0

if [[ ! -o interactive ]]; then
    return 0
fi

# The installed wrapper exports this path; ordinary stock Zsh sessions do not.
if [[ -z ${KEEL_MODULE_PATH:-} ]]; then
    return 0
fi

zmodload -i zsh/zle || return 1
autoload -Uz add-zle-hook-widget || return 1

if [[ -n ${KEEL_MODULE_PATH:-} ]]; then
    module_path=("$KEEL_MODULE_PATH" ${module_path:-})
fi

if ! zmodload -i keel; then
    print -u2 'keel: native module unavailable; use scripts/start-keel.sh'
    return 1
fi

add-zle-hook-widget line-init keel-native-line-init
add-zle-hook-widget line-finish keel-native-line-finish
_KEEL_ZSH_HOOKS=1

_keel_disable() {
    if (( _KEEL_ZSH_HOOKS )); then
        add-zle-hook-widget -d line-init keel-native-line-init
        add-zle-hook-widget -d line-finish keel-native-line-finish
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
