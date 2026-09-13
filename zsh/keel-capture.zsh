# Capture native Zsh matches in a forked zpty without allowing ZLE to paint.
_keel_capture_compadd() {
    local result arg description_source label candidate detail key display suffix index
    local count i
    local -a original filtered raw generated descriptions

    original=("$@")

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

    # Ask Zsh to produce both display labels and insertion candidates in one
    # pass; repeating compadd triples provider-side work for large lists.
    builtin compadd -O raw -A generated "${filtered[@]}" >/dev/null 2>&1
    result=$?

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
        # _describe may add a plain row before its formatted description.
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

_keel_completion_broad_context() {
    emulate -L zsh

    local current_buffer=$1
    local current_cursor=$2
    local line_prefix token_prefix context_prefix
    local rest_after_cursor token_suffix line_suffix option_prefix

    line_prefix=${current_buffer[1,current_cursor]}
    token_prefix=${line_prefix##*[[:space:]]}
    if (( ${#token_prefix} < ${#line_prefix} )); then
        context_prefix=${line_prefix[1,$(( ${#line_prefix} - ${#token_prefix} ))]}
    else
        context_prefix=''
    fi
    rest_after_cursor=${current_buffer[$(( current_cursor + 1 )),-1]}
    token_suffix=${rest_after_cursor%%[[:space:]]*}
    line_suffix=${rest_after_cursor#${token_suffix}}
    if [[ $token_prefix == --* ]]; then
        option_prefix='--'
    elif [[ $token_prefix == -* ]]; then
        option_prefix='-'
    else
        option_prefix=''
    fi
    typeset -g _KEEL_COMPLETION_BROAD_BUFFER="${context_prefix}${option_prefix}${line_suffix}"
    typeset -g _KEEL_COMPLETION_BROAD_CURSOR=$(( ${#context_prefix} + ${#option_prefix} ))
}

_keel_completion_capture_sync() {
    emulate -L zsh
    setopt localoptions no_monitor

    local request=$1
    local field=$'\x1f'
    local record=$'\x1e'
    local end=$'\x1d'
    local completion_widget payload item
    local current_buffer current_cursor broad_buffer broad_cursor
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

    current_buffer=$BUFFER
    current_cursor=$CURSOR
    if [[ -n $completion_widget ]]; then
        if [[ -n ${EPOCHREALTIME:-} ]]; then
            started=$EPOCHREALTIME
        else
            started=$SECONDS
        fi
        # Capture a broad result set without the active token so the core
        # fuzzy matcher can handle non-prefix queries.
        _keel_completion_broad_context "$current_buffer" "$current_cursor"
        broad_buffer=$_KEEL_COMPLETION_BROAD_BUFFER
        broad_cursor=$_KEEL_COMPLETION_BROAD_CURSOR
        BUFFER=$broad_buffer
        CURSOR=$broad_cursor
        zle -- "$completion_widget" >/dev/null 2>&1
        BUFFER=$current_buffer
        CURSOR=$current_cursor
        if [[ -n ${EPOCHREALTIME:-} ]]; then
            finished=$EPOCHREALTIME
            elapsed_ms=$(( (finished - started) * 1000 ))
        else
            elapsed_ms=$(( (SECONDS - started) * 1000 ))
        fi
    fi
    (( elapsed_ms < 0 )) && elapsed_ms=0

    stty -onlcr -ocrnl </dev/tty >/dev/null 2>&1 || true
    payload="K1${field}${request}${field}${elapsed_ms}${record}"
    for item in "${_KEEL_CAPTURE_RECORDS[@]}"; do
        payload+=$item
    done
    print -rn -- "${payload}${end}"
}
