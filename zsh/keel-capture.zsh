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
    typeset -g _KEEL_COMPLETION_LONG_OPTION_PROBE=0
    if [[ $token_prefix == --* ]]; then
        typeset -g _KEEL_COMPLETION_LONG_OPTION_PROBE=1
    fi
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

_keel_completion_prepare_provider() {
    emulate -L zsh

    local command_name=$1
    local provider=${_comps[$command_name]-}

    # macOS grep advertises the long options that _grep hides for its BSD
    # variant.  Keel asks for a complete long-option set, so keep that
    # provider's own descriptions instead of truncating the API result.
    if [[ $OSTYPE == darwin* && $command_name == grep && $provider == _grep ]]; then
        typeset -gA _cmd_variant
        _cmd_variant[$command_name]=gnu
    fi
}

_keel_completion_capture_sync() {
    emulate -L zsh
    setopt localoptions no_monitor

    local request=$1
    local completion_widget
    local current_buffer current_cursor broad_buffer broad_cursor command_name
    local -F started finished
    integer elapsed_tenths_ms=0
    typeset -g _KEEL_COMPLETION_CAPTURE=1

    _keel_completion_capture_post() {
        compstate[insert]=''
        compstate[list]=''
    }
    local -a saved_comppostfuncs
    saved_comppostfuncs=("${comppostfuncs[@]}")
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
        command_name=${${(@z)broad_buffer}[1]}
        command_name=${command_name:t}
        _keel_completion_prepare_provider "$command_name"
        zstyle ':completion:*' verbose yes
        zstyle ':completion:*' list-grouped no
        BUFFER=$broad_buffer
        CURSOR=$broad_cursor
        zle -- "$completion_widget" >/dev/null 2>&1
        comppostfuncs=("${saved_comppostfuncs[@]}")
        BUFFER=$current_buffer
        CURSOR=$current_cursor
        if [[ -n ${EPOCHREALTIME:-} ]]; then
            finished=$EPOCHREALTIME
            elapsed_tenths_ms=$(( (finished - started) * 10000 ))
        else
            elapsed_tenths_ms=$(( (SECONDS - started) * 10000 ))
        fi
    fi
    (( elapsed_tenths_ms < 0 )) && elapsed_tenths_ms=0

    zle keel-native-finish-capture "$request" "$elapsed_tenths_ms"
}
