# Private command boundaries for the PTY host. Payloads are encoded so command
# text cannot terminate an OSC sequence or be mistaken for protocol fields.
_sopra_host_active() {
    [[ ${_SOPRA_HOSTED:-} == 1 && ${_SOPRA_HOST_SHELL_PID:-0} == $$ ]]
}

_sopra_host_hex() {
    emulate -L zsh
    local LC_ALL=C value=$1 byte encoded=''
    local -i index
    for (( index=1; index<=${#value}; index++ )); do
        printf -v byte '%02x' "'${value[index]}"
        encoded+=$byte
    done
    REPLY=$encoded
}

_sopra_host_begin() {
    _sopra_host_active || return 0
    printf '\e]777;sopra;executing\a'
    _sopra_host_hex "$1"
    local command=$REPLY
    _sopra_host_hex "$PWD"
    printf '\e]777;sopra;begin;%s;%s\a' "$command" "$REPLY"
}

