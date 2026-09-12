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
