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
    local -i arm_completion=${1:-0}
    local -i suppress_completion=${2:-0}
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
    local -a editor_args
    editor_args=(
        --buffer "$BUFFER" \
        --cursor "$CURSOR" \
        --prompt "$prompt" \
        --cwd "$PWD"
    )
    if (( arm_completion )); then
        editor_args+=(--arm-completion)
    fi
    if (( suppress_completion )); then
        editor_args+=(--suppress-completion)
    fi
    raw=$("$SOPRA_BIN" "${editor_args[@]}" </dev/tty) || return 1
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
    local -i arm_completion=0 suppress_completion=0
    local previous_buffer previous_cursor previous_history
    while _sopra_edit "$arm_completion" "$suppress_completion"; do
        arm_completion=0
        suppress_completion=0
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
                previous_buffer=$BUFFER
                previous_cursor=$CURSOR
                previous_history=${HISTNO:-}
                zle up-line-or-history
                if [[ $BUFFER == $previous_buffer && $CURSOR == $previous_cursor \
                    && ${HISTNO:-} == $previous_history ]]; then
                    arm_completion=1
                else
                    suppress_completion=1
                fi
                ;;
            down)
                previous_buffer=$BUFFER
                previous_cursor=$CURSOR
                previous_history=${HISTNO:-}
                zle down-line-or-history
                if [[ $BUFFER == $previous_buffer && $CURSOR == $previous_cursor \
                    && ${HISTNO:-} == $previous_history ]]; then
                    arm_completion=1
                else
                    suppress_completion=1
                fi
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
