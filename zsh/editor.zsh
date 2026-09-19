if [[ -n ${_SOPRA_LOADED:-} ]]; then
    return 0
fi
typeset -g _SOPRA_LOADED=1

if [[ ! -o interactive ]]; then
    return 0
fi

typeset -r script_dir=${${(%):-%N}:A:h}
if [[ -z ${SOPRA_BIN:-} ]]; then
    if (( ${+commands[sopra]} )); then
        typeset -g SOPRA_BIN=$commands[sopra]
    elif [[ -x $script_dir/../target/debug/sopra ]]; then
        typeset -g SOPRA_BIN=$script_dir/../target/debug/sopra
    else
        typeset -g SOPRA_BIN=$script_dir/../../bin/sopra
    fi
fi
: "${SOPRA_PROMPT:=${PROMPT:-%n in %~ $ }}"
: "${SOPRA_RPROMPT:=${RPROMPT:-}}"
: "${SOPRA_TRANSIENT_PROMPT:=$ }"

if [[ ! -x $SOPRA_BIN ]]; then
    print -u2 "sopra: executable not found at $SOPRA_BIN"
    return 1
fi

typeset -g _SOPRA_RESULT_ACTION=''
typeset -g _SOPRA_RESULT_BUFFER=''
typeset -g _SOPRA_RESULT_CURSOR=0
typeset -g _SOPRA_RESULT_HIGHLIGHTS=''

_sopra_hex_decode() {
    emulate -L zsh
    local hex=$1 escaped='' pair
    (( ${#hex} % 2 == 0 )) || return 1
    while [[ -n $hex ]]; do
        pair=${hex[1,2]}
        [[ $pair == [[:xdigit:]][[:xdigit:]] ]] || return 1
        escaped+="\\x$pair"
        hex=${hex[3,-1]}
    done
    typeset -g _SOPRA_RESULT_BUFFER
    printf -v _SOPRA_RESULT_BUFFER '%b' "$escaped"
}

_sopra_parse_result() {
    emulate -L zsh
    local raw=$1
    local -a fields
    IFS=$'\t' read -rA fields <<< "$raw"
    [[ ${fields[1]-} == K1 && ${fields[2]-} == (accept|interrupt|up|down|tab|eof) ]] || return 1
    [[ ${fields[3]-} == <-> ]] || return 1
    _sopra_hex_decode "${fields[4]-}" || return 1
    typeset -g _SOPRA_RESULT_ACTION=${fields[2]}
    typeset -g _SOPRA_RESULT_CURSOR=${fields[3]}
    typeset -g _SOPRA_RESULT_HIGHLIGHTS=${fields[5]-}
}

_sopra_edit() {
    emulate -L zsh
    local raw prompt
    local -x SOPRA_ALIASES SOPRA_FUNCTIONS SOPRA_COMMANDS SOPRA_VARIABLES SOPRA_ARRAYS
    printf -v SOPRA_ALIASES '%s\n' "${(@k)aliases}"
    printf -v SOPRA_FUNCTIONS '%s\n' "${(@k)functions}"
    printf -v SOPRA_VARIABLES '%s\n' "${(@k)parameters}"
    local -a arrays
    local variable
    for variable in ${(k)parameters}; do
        if [[ ${(tP)variable} == *array* ]]; then
            arrays+=("$variable")
        fi
    done
    printf -v SOPRA_ARRAYS '%s\n' "${arrays[@]}"
    local -a command_list
    local command_name
    for command_name in ${(k)commands}; do
        command_list+=("$command_name"$'\t'"${commands[$command_name]}")
    done
    printf -v SOPRA_COMMANDS '%s\n' "${command_list[@]}"
    prompt=$(print -P -- "$SOPRA_PROMPT")
    raw=$("$SOPRA_BIN" \
        --buffer "$BUFFER" \
        --cursor "$CURSOR" \
        --prompt "$prompt" \
        --cwd "$PWD" \
        </dev/tty) || return 1
    _sopra_parse_result "$raw"
}

_sopra_prepare_transient() {
    PROMPT=$SOPRA_TRANSIENT_PROMPT
    RPROMPT=''
}

_sopra_clear_highlights() {
    region_highlight=()
}

_sopra_line_init() {
    PROMPT=$SOPRA_PROMPT
    RPROMPT=$SOPRA_RPROMPT
    _sopra_clear_highlights
    while _sopra_edit; do
        BUFFER=$_SOPRA_RESULT_BUFFER
        CURSOR=$_SOPRA_RESULT_CURSOR
        case $_SOPRA_RESULT_ACTION in
            accept)
                _sopra_prepare_transient
                if [[ -n $_SOPRA_RESULT_HIGHLIGHTS ]]; then
                    region_highlight=("${(@s.;.)_SOPRA_RESULT_HIGHLIGHTS}")
                fi
                zle .accept-line
                return 0
                ;;
            interrupt)
                _sopra_prepare_transient
                _sopra_clear_highlights
                BUFFER=''
                CURSOR=0
                zle .accept-line
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

zle -N zle-line-init _sopra_line_init
