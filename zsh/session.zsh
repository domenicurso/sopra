_sopra_server_start() {
    emulate -L zsh
    if [[ -n ${_SOPRA_SERVER_PID:-} ]] && kill -0 "$_SOPRA_SERVER_PID" 2>/dev/null; then
        return 0
    fi
    [[ -n ${_SOPRA_SERVER_DIR:-} ]] && _sopra_server_stop
    zmodload zsh/net/socket || return 1
    zmodload zsh/zselect || return 1
    typeset -g _SOPRA_SERVER_DIR=$(mktemp -d "${TMPDIR:-/tmp}/sopra.XXXXXXXX")
    [[ -d $_SOPRA_SERVER_DIR ]] || return 1
    typeset -g _SOPRA_SERVER_SOCKET=$_SOPRA_SERVER_DIR/socket
    "$SOPRA_BIN" --serve "$_SOPRA_SERVER_SOCKET" "$$" </dev/null >/dev/null 2>"$_SOPRA_SERVER_DIR/error" &!
    typeset -g _SOPRA_SERVER_PID=$!
    local -i attempt
    for attempt in {1..100}; do
        [[ -S $_SOPRA_SERVER_SOCKET ]] && return 0
        kill -0 "$_SOPRA_SERVER_PID" 2>/dev/null || break
        zselect -t 1
    done
    cat "$_SOPRA_SERVER_DIR/error" >&2
    _sopra_server_stop
    return 1
}

_sopra_worker_session() {
    emulate -L zsh
    local request raw
    local -i socket_fd
    request=$(mktemp "$_SOPRA_SERVER_DIR/request.XXXXXXXX") || return 1
    {
        { printf 'S1\0%s\0' "$#"; printf '%s\0' "$@"; command env -0; } > "$request"
        zsocket "$_SOPRA_SERVER_SOCKET" || return 1
        socket_fd=$REPLY
        print -r -u "$socket_fd" -- "$request"
        IFS= read -r -u "$socket_fd" raw || return 1
        if [[ $raw != K2$'\t'* ]]; then
            print -u2 -- "sopra: $raw"
            return 1
        fi
        print -r -- "$raw"
    } always {
        (( socket_fd )) && exec {socket_fd}>&-
        command rm -f -- "$request"
    }
}

_sopra_server_stop() {
    (( ZSH_SUBSHELL == 0 )) || return 0
    [[ -n ${_SOPRA_SERVER_PID:-} ]] && kill "$_SOPRA_SERVER_PID" 2>/dev/null
    [[ -n ${_SOPRA_SERVER_DIR:-} ]] && command rm -rf -- "$_SOPRA_SERVER_DIR"
    unset _SOPRA_SERVER_PID _SOPRA_SERVER_DIR _SOPRA_SERVER_SOCKET
}
autoload -Uz add-zsh-hook
add-zsh-hook zshexit _sopra_server_stop
