if [[ -n ${_KEEL_LOADED:-} ]]; then
    return 0
fi
typeset -g _KEEL_LOADED=1

if [[ ! -o interactive ]]; then
    return 0
fi

typeset -r keel_dir=${${(%):-%N}:A:h}
: "${KEEL_BIN:=$keel_dir/../target/debug/keel}"
: "${KEEL_PROMPT:=${PROMPT:-%n in %~ ❯ }}"
: "${KEEL_RPROMPT:=${RPROMPT:-}}"
: "${KEEL_TRANSIENT_PROMPT:=❯ }"

if [[ ! -x $KEEL_BIN ]]; then
    print -u2 "keel: executable not found at $KEEL_BIN"
    return 1
fi

typeset -g _KEEL_RESULT_ACTION=''
typeset -g _KEEL_RESULT_BUFFER=''
typeset -g _KEEL_RESULT_CURSOR=0

_keel_hex_decode() {
    emulate -L zsh
    local hex=$1 escaped='' pair
    (( ${#hex} % 2 == 0 )) || return 1
    while [[ -n $hex ]]; do
        pair=${hex[1,2]}
        [[ $pair == [[:xdigit:]][[:xdigit:]] ]] || return 1
        escaped+="\\x$pair"
        hex=${hex[3,-1]}
    done
    typeset -g _KEEL_RESULT_BUFFER
    printf -v _KEEL_RESULT_BUFFER '%b' "$escaped"
}

_keel_parse_result() {
    emulate -L zsh
    local raw=$1
    local -a fields
    IFS=$'\t' read -rA fields <<< "$raw"
    [[ ${fields[1]-} == K1 && ${fields[2]-} == (accept|cancel|interrupt|up|down|tab|eof) ]] || return 1
    [[ ${fields[3]-} == <-> ]] || return 1
    _keel_hex_decode "${fields[4]-}" || return 1
    typeset -g _KEEL_RESULT_ACTION=${fields[2]}
    typeset -g _KEEL_RESULT_CURSOR=${fields[3]}
}

_keel_edit() {
    emulate -L zsh
    local raw prompt
    local -x FPATH="${(j.:.)fpath}"
    local -x KEEL_ALIASES="${(j:\n:)${(k)aliases}}"
    local -x KEEL_FUNCTIONS="${(j:\n:)${(k)functions}}"
    prompt=$(print -P -- "$KEEL_PROMPT")
    raw=$("$KEEL_BIN" \
        --buffer "$BUFFER" \
        --cursor "$CURSOR" \
        --prompt "$prompt" \
        --cwd "$PWD" \
        </dev/tty) || return 1
    _keel_parse_result "$raw"
}

_keel_prepare_transient() {
    PROMPT=$KEEL_TRANSIENT_PROMPT
    RPROMPT=''
}

_keel_line_init() {
    PROMPT=$KEEL_PROMPT
    RPROMPT=$KEEL_RPROMPT
    while _keel_edit; do
        BUFFER=$_KEEL_RESULT_BUFFER
        CURSOR=$_KEEL_RESULT_CURSOR
        case $_KEEL_RESULT_ACTION in
            accept)
                _keel_prepare_transient
                zle .accept-line
                return 0
                ;;
            interrupt)
                _keel_prepare_transient
                BUFFER=''
                CURSOR=0
                zle .accept-line
                return 0
                ;;
            cancel)
                zle -R
                return 0
                ;;
            up)
                zle up-line-or-history
                ;;
            down)
                zle down-line-or-history
                ;;
            tab)
                zle expand-or-complete
                ;;
            eof)
                zle .eof
                return 0
                ;;
        esac
    done
    zle -R
}

zle -N zle-line-init _keel_line_init
