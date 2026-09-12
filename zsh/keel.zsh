if [[ -n ${_KEEL_ZSH_LOADED:-} ]]; then
    return 0
fi

typeset -g _KEEL_ZSH_LOADED=1
typeset -g _KEEL_ZSH_HOOKS=0
typeset -g _KEEL_ZSH_KEYS=0
typeset -gA _KEEL_SAVED_TAB_BINDINGS
typeset -gA _KEEL_SAVED_UP_BINDINGS
typeset -gA _KEEL_SAVED_DOWN_BINDINGS
typeset -gA _KEEL_SAVED_ESCAPE_BINDINGS
typeset -gA _KEEL_SAVED_INTERRUPT_BINDINGS
typeset -gA _KEEL_SAVED_ENTER_BINDINGS
typeset -gA _KEEL_SAVED_LINEFEED_BINDINGS
typeset -g _KEEL_COMPLETION_PTY=''
typeset -g _KEEL_COMPLETION_FD=''
typeset -g _KEEL_COMPLETION_REQUEST=0
typeset -g _KEEL_COMPLETION_PENDING_ID=''
typeset -g _KEEL_COMPLETION_PENDING_BUFFER=''
typeset -g _KEEL_COMPLETION_PENDING_CURSOR=''
typeset -g _KEEL_COMPLETION_PENDING_PWD=''
typeset -g _KEEL_COMPLETION_LAST_ID=''
typeset -g _KEEL_COMPLETION_LAST_BUFFER=''
typeset -g _KEEL_COMPLETION_LAST_CURSOR=''
typeset -g _KEEL_COMPLETION_LAST_PWD=''
typeset -g _KEEL_COMPLETION_RESPONSE_ID=''
typeset -g _KEEL_COMPLETION_RESPONSE_BUFFER=''
typeset -g _KEEL_COMPLETION_RESPONSE_CURSOR=''
typeset -g _KEEL_COMPLETION_RESPONSE_PWD=''
typeset -g _KEEL_COMPLETION_RESPONSE_PAYLOAD=''
typeset -g _KEEL_COMPLETION_RESPONSE_MS=0
typeset -g _KEEL_COMPLETION_CAPTURE=0
typeset -gr _KEEL_MAX_COMPLETIONS=512

if [[ ! -o interactive ]]; then
    return 0
fi

# The installed wrapper exports this path; ordinary stock Zsh sessions do not.
if [[ -z ${KEEL_MODULE_PATH:-} ]]; then
    return 0
fi

zmodload -i zsh/zle || return 1
zmodload -i zsh/parameter || return 1
zmodload -i zsh/datetime || true
zmodload -i zsh/zpty || true
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
zle -N _keel_accept_widget
zle -N _keel_select_previous_widget
zle -N _keel_select_next_widget
zle -N _keel_dismiss_widget
zle -N _keel_accept_enter_widget
zle -N _keel_accept_linefeed_widget
zle -N _keel_completion_apply_widget
_KEEL_ZSH_HOOKS=1

# This wrapper runs only in the forked zpty used for capture. It lets the real
# completion functions populate their normal match table while copying the
# matched and generated values into the bounded Keel protocol.
_keel_capture_compadd() {
    local result arg description_source label candidate detail key display suffix index
    local count i
    local -a original filtered raw generated descriptions

    original=("$@")
    builtin compadd "$@"
    result=$?

    filtered=()
    for (( i = 1; i <= $#original; i++ )); do
        arg=${original[i]}
        case "$arg" in
            -O|-A)
                (( i++ ))
                ;;
            -O*|-A*)
                ;;
            *)
                filtered+=("$arg")
                ;;
        esac
    done

    builtin compadd -O raw "${filtered[@]}" >/dev/null 2>&1 || true
    builtin compadd -A generated "${filtered[@]}" >/dev/null 2>&1 || true

    for (( i = 1; i <= $#original; i++ )); do
        arg=${original[i]}
        case "$arg" in
            -d)
                description_source=${original[i + 1]}
                (( i++ ))
                ;;
            -d\(*\))
                description_source=${arg#-d}
                ;;
        esac
    done

    if [[ -n $description_source ]]; then
        if [[ $description_source == \(*\) ]]; then
            descriptions=( "${(@z)description_source}" )
            if (( $#descriptions >= 2 )) && [[ ${descriptions[1]} == '(' && ${descriptions[-1]} == ')' ]]; then
                descriptions=( "${(@)descriptions[2,-2]}" )
            fi
        else
            descriptions=( "${(@P)description_source}" )
            (( $#descriptions )) || descriptions=("$description_source")
        fi
    fi

    count=$#raw
    (( $#generated > count )) && count=$#generated
    for (( i = 1; i <= count && ${#_KEEL_CAPTURE_RECORDS[@]} < _KEEL_MAX_COMPLETIONS; i++ )); do
        label=${raw[i]:-${generated[i]}}
        candidate=${generated[i]:-${raw[i]}}
        detail=${descriptions[i]:-}
        [[ -n $label && -n $candidate ]] || continue
        if [[ $label == *$'\x1d'* || $label == *$'\x1e'* || $label == *$'\x1f'* ||
              $label == *$'\r'* || $label == *$'\n'* ||
              $candidate == *$'\x1d'* || $candidate == *$'\x1e'* ||
              $candidate == *$'\x1f'* || $candidate == *$'\r'* ||
              $candidate == *$'\n'* || $detail == *$'\x1d'* ||
              $detail == *$'\x1e'* || $detail == *$'\x1f'* ||
              $detail == *$'\r'* || $detail == *$'\n'* ]]; then
            continue
        fi
        # _describe may first add a plain match and then add the same match
        # again with its formatted description. Keep one row and prefer the
        # descriptive record when it arrives.
        display=$detail
        if [[ -n $display && $display == "$label"* ]]; then
            suffix=${display#$label}
            if [[ $suffix == *'-- '* ]]; then
                detail=${suffix#*-- }
                detail=${detail##[[:space:]]#}
                detail=${detail%%[[:space:]]#}
            fi
        fi
        key=$candidate
        if [[ -n ${_KEEL_CAPTURE_SEEN[$key]:-} ]]; then
            index=${_KEEL_CAPTURE_SEEN[$key]}
            if [[ -z ${_KEEL_CAPTURE_SEEN_DETAIL[$key]:-} && -n $detail ]]; then
                _KEEL_CAPTURE_RECORDS[index]="${label}"$'\x1f'"${detail}"$'\x1f'"${candidate}"$'\x1e'
                _KEEL_CAPTURE_SEEN_DETAIL[$key]=$detail
            fi
            continue
        fi
        _KEEL_CAPTURE_SEEN[$key]=$(( $#_KEEL_CAPTURE_RECORDS + 1 ))
        _KEEL_CAPTURE_SEEN_DETAIL[$key]=$detail
        _KEEL_CAPTURE_RECORDS+=("${label}"$'\x1f'"${detail}"$'\x1f'"${candidate}"$'\x1e')
    done

    return "$result"
}

_keel_completion_capture_sync() {
    emulate -L zsh
    setopt localoptions no_monitor

    local request=$1
    local field=$'\x1f'
    local record=$'\x1e'
    local end=$'\x1d'
    local completion_widget payload item
    local -F started finished
    integer elapsed_ms=0
    typeset -ga _KEEL_CAPTURE_RECORDS
    _KEEL_CAPTURE_RECORDS=()
    typeset -gA _KEEL_CAPTURE_SEEN _KEEL_CAPTURE_SEEN_DETAIL
    _KEEL_CAPTURE_SEEN=()
    _KEEL_CAPTURE_SEEN_DETAIL=()
    typeset -g _KEEL_COMPLETION_CAPTURE=1

    compadd() { _keel_capture_compadd "$@"; }
    _keel_completion_capture_post() {
        compstate[insert]=''
        unset 'compstate[list]'
    }
    local -a +h comppostfuncs
    comppostfuncs=(_keel_completion_capture_post)
    completion_widget=${(k)widgets[(r)completion:.complete-word:_main_complete]}

    if [[ -n $completion_widget ]]; then
        if [[ -n ${EPOCHREALTIME:-} ]]; then
            started=$EPOCHREALTIME
        else
            started=$SECONDS
        fi
        zle -- "$completion_widget" >/dev/null 2>&1
        if [[ -n ${EPOCHREALTIME:-} ]]; then
            finished=$EPOCHREALTIME
            elapsed_ms=$(( (finished - started) * 1000 ))
        else
            elapsed_ms=$(( (SECONDS - started) * 1000 ))
        fi
    fi
    (( elapsed_ms < 0 )) && elapsed_ms=0

    # Keep the protocol byte-for-byte stable across the pty line discipline.
    stty -onlcr -ocrnl </dev/tty >/dev/null 2>&1 || true
    payload="K1${field}${request}${field}${elapsed_ms}${record}"
    for item in "${_KEEL_CAPTURE_RECORDS[@]}"; do
        payload+=$item
    done
    print -rn -- "${payload}${end}"
}

_keel_completion_stop_pending() {
    if [[ -n $_KEEL_COMPLETION_FD ]]; then
        zle -F "$_KEEL_COMPLETION_FD" 2>/dev/null || true
    fi
    if [[ -n $_KEEL_COMPLETION_PTY ]]; then
        zpty -d "$_KEEL_COMPLETION_PTY" 2>/dev/null || true
    fi
    _KEEL_COMPLETION_FD=''
    _KEEL_COMPLETION_PTY=''
    _KEEL_COMPLETION_PENDING_ID=''
    _KEEL_COMPLETION_PENDING_BUFFER=''
    _KEEL_COMPLETION_PENDING_CURSOR=''
    _KEEL_COMPLETION_PENDING_PWD=''
}

_keel_completion_response() {
    local fd=$1
    local pty=$_KEEL_COMPLETION_PTY
    local field=$'\x1f'
    local record=$'\x1e'
    local end=$'\x1d'
    local frame header metadata request elapsed payload
    local pending_id pending_buffer pending_cursor pending_pwd

    [[ -n $pty && $fd == $_KEEL_COMPLETION_FD ]] || return 0
    pending_id=$_KEEL_COMPLETION_PENDING_ID
    pending_buffer=$_KEEL_COMPLETION_PENDING_BUFFER
    pending_cursor=$_KEEL_COMPLETION_PENDING_CURSOR
    pending_pwd=$_KEEL_COMPLETION_PENDING_PWD

    if ! zpty -r "$pty" frame "*${end}"; then
        _KEEL_COMPLETION_LAST_ID=$pending_id
        _KEEL_COMPLETION_LAST_BUFFER=$pending_buffer
        _KEEL_COMPLETION_LAST_CURSOR=$pending_cursor
        _KEEL_COMPLETION_LAST_PWD=$pending_pwd
        _keel_completion_stop_pending
        return 0
    fi
    frame=${frame//$'\r'/}
    header=${frame%%${record}*}
    [[ $header == K1${field}* ]] || {
        _keel_completion_stop_pending
        return 0
    }
    metadata=${header#K1${field}}
    request=${metadata%%${field}*}
    elapsed=${metadata#*${field}}
    payload=${frame#*${record}}
    payload=${payload%$end}
    [[ $request == "$pending_id" && $elapsed == <-> ]] || {
        _keel_completion_stop_pending
        return 0
    }

    _KEEL_COMPLETION_LAST_ID=$pending_id
    _KEEL_COMPLETION_LAST_BUFFER=$pending_buffer
    _KEEL_COMPLETION_LAST_CURSOR=$pending_cursor
    _KEEL_COMPLETION_LAST_PWD=$pending_pwd
    _keel_completion_stop_pending

    _KEEL_COMPLETION_RESPONSE_ID=$pending_id
    _KEEL_COMPLETION_RESPONSE_BUFFER=$pending_buffer
    _KEEL_COMPLETION_RESPONSE_CURSOR=$pending_cursor
    _KEEL_COMPLETION_RESPONSE_PWD=$pending_pwd
    _KEEL_COMPLETION_RESPONSE_PAYLOAD=$payload
    _KEEL_COMPLETION_RESPONSE_MS=$elapsed
    zle _keel_completion_apply_widget
}

_keel_completion_apply_widget() {
    if [[ $BUFFER == "$_KEEL_COMPLETION_RESPONSE_BUFFER" &&
          $CURSOR == "$_KEEL_COMPLETION_RESPONSE_CURSOR" &&
          $PWD == "$_KEEL_COMPLETION_RESPONSE_PWD" ]]; then
        zle keel-native-set-suggestions "K$_KEEL_COMPLETION_RESPONSE_PAYLOAD" "$_KEEL_COMPLETION_RESPONSE_MS" || true
    fi
    zle -R
}

_keel_completion_request() {
    local current_buffer=$BUFFER
    local current_cursor=$CURSOR
    local current_pwd=$PWD
    local request

    if [[ $current_buffer == "$_KEEL_COMPLETION_PENDING_BUFFER" &&
          $current_cursor == "$_KEEL_COMPLETION_PENDING_CURSOR" &&
          $current_pwd == "$_KEEL_COMPLETION_PENDING_PWD" &&
          -n $_KEEL_COMPLETION_PTY ]]; then
        return 0
    fi
    if [[ $current_buffer == "$_KEEL_COMPLETION_LAST_BUFFER" &&
          $current_cursor == "$_KEEL_COMPLETION_LAST_CURSOR" &&
          $current_pwd == "$_KEEL_COMPLETION_LAST_PWD" ]]; then
        return 0
    fi

    _keel_completion_stop_pending
    _KEEL_COMPLETION_LAST_ID=''
    _KEEL_COMPLETION_LAST_BUFFER=''
    _KEEL_COMPLETION_LAST_CURSOR=''
    _KEEL_COMPLETION_LAST_PWD=''
    [[ -n $current_buffer ]] || return 0
    whence -w compdef >/dev/null 2>&1 || return 0
    whence -w _main_complete >/dev/null 2>&1 || return 0
    whence -w zpty >/dev/null 2>&1 || return 0

    (( _KEEL_COMPLETION_REQUEST++ ))
    request=$_KEEL_COMPLETION_REQUEST
    _KEEL_COMPLETION_PTY=_keel_completion_pty
    if ! zpty -b "$_KEEL_COMPLETION_PTY" _keel_completion_capture_sync "$request"; then
        _KEEL_COMPLETION_PTY=''
        return 0
    fi
    _KEEL_COMPLETION_FD=$REPLY
    _KEEL_COMPLETION_PENDING_ID=$request
    _KEEL_COMPLETION_PENDING_BUFFER=$current_buffer
    _KEEL_COMPLETION_PENDING_CURSOR=$current_cursor
    _KEEL_COMPLETION_PENDING_PWD=$current_pwd
    zle -F "$_KEEL_COMPLETION_FD" _keel_completion_response || _keel_completion_stop_pending
}

_keel_completion_pre_redraw() {
    (( _KEEL_ZSH_HOOKS && ! _KEEL_COMPLETION_CAPTURE )) || return 0
    _keel_completion_request
}

_keel_completion_line_init() {
    _keel_completion_stop_pending
    _KEEL_COMPLETION_LAST_ID=''
    _KEEL_COMPLETION_LAST_BUFFER=''
    _KEEL_COMPLETION_LAST_CURSOR=''
    _KEEL_COMPLETION_LAST_PWD=''
}

_keel_completion_line_finish() {
    _keel_completion_line_init
}

add-zle-hook-widget line-pre-redraw _keel_completion_pre_redraw
add-zle-hook-widget line-init _keel_completion_line_init
add-zle-hook-widget line-finish _keel_completion_line_finish

_keel_original_binding() {
    local map=${KEYMAP:-main}
    case "$1" in
        tab)
            print -r -- "${_KEEL_SAVED_TAB_BINDINGS[$map]:-undefined-key}"
            ;;
        up)
            print -r -- "${_KEEL_SAVED_UP_BINDINGS[$map]:-undefined-key}"
            ;;
        down)
            print -r -- "${_KEEL_SAVED_DOWN_BINDINGS[$map]:-undefined-key}"
            ;;
        escape)
            print -r -- "${_KEEL_SAVED_ESCAPE_BINDINGS[$map]:-undefined-key}"
            ;;
        enter)
            print -r -- "${_KEEL_SAVED_ENTER_BINDINGS[$map]:-accept-line}"
            ;;
        linefeed)
            print -r -- "${_KEEL_SAVED_LINEFEED_BINDINGS[$map]:-accept-line}"
            ;;
    esac
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
        _keel_bind_key "$map" '^C' keel-native-clear-line _KEEL_SAVED_INTERRUPT_BINDINGS
        _keel_bind_key "$map" '^M' _keel_accept_enter_widget _KEEL_SAVED_ENTER_BINDINGS
        _keel_bind_key "$map" '^J' _keel_accept_linefeed_widget _KEEL_SAVED_LINEFEED_BINDINGS
    done
    _KEEL_ZSH_KEYS=1
}

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

_keel_bind_keys

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
