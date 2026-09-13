_keel_completion_stop_pending() {
    if [[ -n $_KEEL_COMPLETION_FD ]]; then
        zle -F "$_KEEL_COMPLETION_FD" 2>/dev/null || true
    fi
    zle keel-native-stop-completion 2>/dev/null || true
    _KEEL_COMPLETION_FD=''
    _KEEL_COMPLETION_PENDING_ID=''
    _KEEL_COMPLETION_PENDING_BUFFER=''
    _KEEL_COMPLETION_PENDING_CURSOR=''
    _KEEL_COMPLETION_PENDING_PWD=''
    _KEEL_COMPLETION_PENDING_CACHE_KEY=''
    _KEEL_COMPLETION_NATIVE_CACHE_KEY=''
}

_keel_completion_command_name() {
    emulate -L zsh
    local -a words

    words=("${(@z)1}")
    REPLY=${words[1]}
}

_keel_completion_load_generated() {
    emulate -L zsh
    local command_name=${1:t}
    local generated provider

    if (( $+_comps[$command_name] )); then
        provider=${_comps[$command_name]}
        # Generated CLI providers commonly occupy the conventional
        # _command name as an autoload placeholder.  Load their generated
        # script before treating the completion as fully initialized.
        if [[ $provider != _${command_name} ||
              ${functions[$provider]-} != *'builtin autoload -XU'* ]]; then
            return 0
        fi
    fi
    (( $+_KEEL_COMPLETION_PROVIDER_TRIED[$command_name] )) && return 1
    _KEEL_COMPLETION_PROVIDER_TRIED[$command_name]=1
    (( $+commands[$command_name] || $+functions[$command_name] )) || return 1
    generated=$("$command_name" completion zsh 2>/dev/null) || return 1
    [[ $generated == *'#compdef'* ]] || return 1
    eval "$generated" 2>/dev/null || return 1
    (( $+_comps[$command_name] ))
}

_keel_completion_request() {
    local current_buffer=$BUFFER
    local current_cursor=$CURSOR
    local current_pwd=$PWD
    local request completion_widget cache_key cached line_prefix

    _KEEL_COMPLETION_LATEST_BUFFER=$current_buffer
    _KEEL_COMPLETION_LATEST_CURSOR=$current_cursor
    _KEEL_COMPLETION_LATEST_PWD=$current_pwd

    if [[ -z $current_buffer ]]; then
        _keel_completion_stop_pending
        _KEEL_COMPLETION_LAST_ID=''
        _KEEL_COMPLETION_LAST_BUFFER=''
        _KEEL_COMPLETION_LAST_CURSOR=''
        _KEEL_COMPLETION_LAST_PWD=''
        _KEEL_COMPLETION_LATEST_BUFFER=''
        _KEEL_COMPLETION_LATEST_CURSOR=0
        _KEEL_COMPLETION_LATEST_PWD=''
        return 0
    fi

    line_prefix=${current_buffer[1,current_cursor]}
    _keel_completion_command_name "$line_prefix"
    [[ -n $REPLY ]] && _keel_completion_load_generated "$REPLY"
    whence -w compdef >/dev/null 2>&1 || return 0
    whence -w _main_complete >/dev/null 2>&1 || return 0

    completion_widget=${(k)widgets[(r)completion:.complete-word:_main_complete]}
    [[ -n $completion_widget ]] || return 0
    _keel_completion_broad_context "$current_buffer" "$current_cursor"
    cache_key="${current_pwd}"$'\x1f'"${completion_widget}"$'\x1f'"${_KEEL_COMPLETION_BROAD_BUFFER}"$'\x1f'"${_KEEL_COMPLETION_BROAD_CURSOR}"$'\x1f'"${_KEEL_COMPLETION_LONG_OPTION_PROBE}"

    if [[ $current_buffer == "$_KEEL_COMPLETION_PENDING_BUFFER" &&
          $current_cursor == "$_KEEL_COMPLETION_PENDING_CURSOR" &&
          $current_pwd == "$_KEEL_COMPLETION_PENDING_PWD" &&
          -n $_KEEL_COMPLETION_FD ]]; then
        return 0
    fi

    # Keep one provider request alive while the user edits the same token;
    # the result is broad enough for every fuzzy query in this context.
    if [[ -n $_KEEL_COMPLETION_FD &&
          $cache_key == "$_KEEL_COMPLETION_PENDING_CACHE_KEY" ]]; then
        return 0
    fi
    if [[ $current_buffer == "$_KEEL_COMPLETION_LAST_BUFFER" &&
          $current_cursor == "$_KEEL_COMPLETION_LAST_CURSOR" &&
          $current_pwd == "$_KEEL_COMPLETION_LAST_PWD" &&
          $cache_key == "$_KEEL_COMPLETION_NATIVE_CACHE_KEY" ]]; then
        return 0
    fi

    _keel_completion_stop_pending
    _KEEL_COMPLETION_LAST_ID=''
    _KEEL_COMPLETION_LAST_BUFFER=''
    _KEEL_COMPLETION_LAST_CURSOR=''
    _KEEL_COMPLETION_LAST_PWD=''

    (( _KEEL_COMPLETION_REQUEST++ ))
    request=$_KEEL_COMPLETION_REQUEST
    if _keel_completion_cache_get "$cache_key"; then
        cached=$REPLY
        _KEEL_COMPLETION_LAST_ID=$request
        _KEEL_COMPLETION_LAST_BUFFER=$current_buffer
        _KEEL_COMPLETION_LAST_CURSOR=$current_cursor
        _KEEL_COMPLETION_LAST_PWD=$current_pwd
        _KEEL_COMPLETION_RESPONSE_ID=$request
        _KEEL_COMPLETION_RESPONSE_BUFFER=$current_buffer
        _KEEL_COMPLETION_RESPONSE_CURSOR=$current_cursor
        _KEEL_COMPLETION_RESPONSE_PWD=$current_pwd
        _KEEL_COMPLETION_RESPONSE_PAYLOAD=$cached
        _KEEL_COMPLETION_RESPONSE_TENTHS_MS=0
        _KEEL_COMPLETION_RESPONSE_CACHE_KEY=$cache_key
        _KEEL_COMPLETION_APPLY_REDRAW=0
        zle _keel_completion_apply_widget
        _KEEL_COMPLETION_APPLY_REDRAW=1
        return 0
    fi
    if ! zle keel-native-start-completion "$request" "$_KEEL_COMPLETION_LONG_OPTION_PROBE"; then
        return 0
    fi
    _KEEL_COMPLETION_FD=$REPLY
    _KEEL_COMPLETION_PENDING_ID=$request
    _KEEL_COMPLETION_PENDING_BUFFER=$current_buffer
    _KEEL_COMPLETION_PENDING_CURSOR=$current_cursor
    _KEEL_COMPLETION_PENDING_PWD=$current_pwd
    _KEEL_COMPLETION_PENDING_CACHE_KEY=$cache_key
    zle -F "$_KEEL_COMPLETION_FD" _keel_completion_response || _keel_completion_stop_pending
}

_keel_completion_pre_redraw() {
    (( _KEEL_ZSH_HOOKS && ! _KEEL_COMPLETION_CAPTURE )) || return 0
    _keel_completion_request
}

_keel_completion_line_init() {
    _keel_completion_stop_pending
    _keel_completion_cache_clear
    _KEEL_COMPLETION_LAST_ID=''
    _KEEL_COMPLETION_LAST_BUFFER=''
    _KEEL_COMPLETION_LAST_CURSOR=''
    _KEEL_COMPLETION_LAST_PWD=''
    _KEEL_COMPLETION_LATEST_BUFFER=''
    _KEEL_COMPLETION_LATEST_CURSOR=0
    _KEEL_COMPLETION_LATEST_PWD=''
    _KEEL_COMPLETION_NATIVE_CACHE_KEY=''
}

_keel_completion_line_finish() {
    _keel_completion_line_init
}
