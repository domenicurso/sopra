_keel_completion_response() {
    local fd=$1
    local field=$'\x1f'
    local record=$'\x1e'
    local end=$'\x1d'
    local frame header metadata request elapsed payload
    local pending_id pending_buffer pending_cursor pending_pwd pending_cache_key
    local response_buffer response_cursor response_pwd current_widget current_cache_key

    [[ -n $_KEEL_COMPLETION_FD && $fd == $_KEEL_COMPLETION_FD ]] || return 0
    pending_id=$_KEEL_COMPLETION_PENDING_ID
    pending_buffer=$_KEEL_COMPLETION_PENDING_BUFFER
    pending_cursor=$_KEEL_COMPLETION_PENDING_CURSOR
    pending_pwd=$_KEEL_COMPLETION_PENDING_PWD
    pending_cache_key=$_KEEL_COMPLETION_PENDING_CACHE_KEY

    zle keel-native-read-completion
    local read_status=$?
    if (( read_status == 1 )); then
        return 0
    fi
    if (( read_status != 0 )); then
        _KEEL_COMPLETION_LAST_ID=$pending_id
        _KEEL_COMPLETION_LAST_BUFFER=$pending_buffer
        _KEEL_COMPLETION_LAST_CURSOR=$pending_cursor
        _KEEL_COMPLETION_LAST_PWD=$pending_pwd
        _keel_completion_stop_pending
        return 0
    fi
    frame=$REPLY
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

    [[ -n $pending_cache_key ]] &&
    _keel_completion_cache_put "$pending_cache_key" "$payload"

    response_buffer=$pending_buffer
    response_cursor=$pending_cursor
    response_pwd=$pending_pwd
    current_widget=${(k)widgets[(r)completion:.complete-word:_main_complete]}
    _keel_completion_broad_context "$_KEEL_COMPLETION_LATEST_BUFFER" "$_KEEL_COMPLETION_LATEST_CURSOR"
    current_cache_key="${_KEEL_COMPLETION_LATEST_PWD}"$'\x1f'"${current_widget}"$'\x1f'"${_KEEL_COMPLETION_BROAD_BUFFER}"$'\x1f'"${_KEEL_COMPLETION_BROAD_CURSOR}"$'\x1f'"${_KEEL_COMPLETION_LONG_OPTION_PROBE}"
    if [[ $current_cache_key == "$pending_cache_key" ]]; then
        response_buffer=$_KEEL_COMPLETION_LATEST_BUFFER
        response_cursor=$_KEEL_COMPLETION_LATEST_CURSOR
        response_pwd=$_KEEL_COMPLETION_LATEST_PWD
        _KEEL_COMPLETION_RESPONSE_CACHE_KEY=$pending_cache_key
    else
        _KEEL_COMPLETION_RESPONSE_CACHE_KEY=''
    fi

    _KEEL_COMPLETION_LAST_ID=$pending_id
    _KEEL_COMPLETION_LAST_BUFFER=$response_buffer
    _KEEL_COMPLETION_LAST_CURSOR=$response_cursor
    _KEEL_COMPLETION_LAST_PWD=$response_pwd
    _keel_completion_stop_pending

    _KEEL_COMPLETION_RESPONSE_ID=$pending_id
    _KEEL_COMPLETION_RESPONSE_BUFFER=$response_buffer
    _KEEL_COMPLETION_RESPONSE_CURSOR=$response_cursor
    _KEEL_COMPLETION_RESPONSE_PWD=$response_pwd
    _KEEL_COMPLETION_RESPONSE_PAYLOAD=$payload
    _KEEL_COMPLETION_RESPONSE_TENTHS_MS=$elapsed
    zle _keel_completion_apply_widget
}

_keel_completion_apply_widget() {
    local applied=0

    if [[ $BUFFER == "$_KEEL_COMPLETION_RESPONSE_BUFFER" &&
          $CURSOR == "$_KEEL_COMPLETION_RESPONSE_CURSOR" &&
          $PWD == "$_KEEL_COMPLETION_RESPONSE_PWD" ]]; then
        if [[ -n $_KEEL_COMPLETION_RESPONSE_CACHE_KEY &&
              $_KEEL_COMPLETION_RESPONSE_CACHE_KEY == "$_KEEL_COMPLETION_NATIVE_CACHE_KEY" ]]; then
            zle keel-native-refresh-suggestions && applied=1
        fi
        if (( ! applied )); then
            zle keel-native-set-suggestions "K$_KEEL_COMPLETION_RESPONSE_PAYLOAD" "$_KEEL_COMPLETION_RESPONSE_TENTHS_MS" && applied=1
        fi
        (( applied )) && _KEEL_COMPLETION_NATIVE_CACHE_KEY=$_KEEL_COMPLETION_RESPONSE_CACHE_KEY
    fi
    (( _KEEL_COMPLETION_APPLY_REDRAW )) && zle -R
}
