if [[ -n ${_SOPRA_LOADED:-} ]]; then
    return 0
fi
typeset -g _SOPRA_LOADED=1

if [[ ! -o interactive ]]; then
    return 0
fi

typeset -r script_dir=${${(%):-%N}:A:h}
if [[ -z ${SOPRA_BIN:-} ]]; then
    if [[ -x $script_dir/../target/debug/sopra-worker ]]; then
        typeset -g SOPRA_BIN=$script_dir/../target/debug/sopra-worker
    elif [[ -x $script_dir/../../libexec/sopra-worker ]]; then
        typeset -g SOPRA_BIN=$script_dir/../../libexec/sopra-worker
    else
        typeset -g SOPRA_BIN=$script_dir/../../libexec/sopra-worker
    fi
fi
source "$script_dir/settings.zsh"
source "$script_dir/protocol.zsh"
_sopra_load_prompt_config
_sopra_load_settings
source "$script_dir/context.zsh"
source "$script_dir/prompt.zsh"
_sopra_prompt_defaults
if [[ -z ${SOPRA_CURSOR_ANIMATION+x} ]]; then
    typeset -g SOPRA_CURSOR_ANIMATION=1
fi
if [[ -z ${SOPRA_SYNTAX_HIGHLIGHTING+x} ]]; then
    typeset -g SOPRA_SYNTAX_HIGHLIGHTING=1
fi
if [[ -z ${SOPRA_COMPLETION_ENABLED+x} ]]; then
    typeset -g SOPRA_COMPLETION_ENABLED=1
fi

if [[ ! -x $SOPRA_BIN ]]; then
    print -u2 "sopra: executable not found at $SOPRA_BIN"
    return 1
fi

_sopra_edit() {
    emulate -L zsh
    local raw expanded_prompt expanded_rprompt expanded_transient
    local -i arm_completion=${1:-0}
    local -i suppress_completion=${2:-0}
    local -x SOPRA_ALIASES SOPRA_FUNCTIONS SOPRA_COMMANDS SOPRA_VARIABLES SOPRA_ARRAYS
    _sopra_shell_context
    expanded_prompt=$(print -P -- "$PROMPT")
    expanded_rprompt=$(print -P -- "$RPROMPT")
    expanded_transient=$(print -P -- "$SOPRA_TRANSIENT_PROMPT")
    local -a editor_args
    editor_args=(
        --buffer "$BUFFER" \
        --cursor "$CURSOR" \
        --prompt "$expanded_prompt" \
        --rprompt "$expanded_rprompt" \
        --cwd "$PWD"
    )
    if (( arm_completion )); then
        editor_args+=(--arm-completion)
    fi
    if (( suppress_completion )); then
        editor_args+=(--suppress-completion)
    fi
    if [[ ${(L)SOPRA_COMPLETION_ENABLED} == (0|false|off|no) ]]; then
        editor_args+=(--disable-completion)
    fi
    if [[ ${(L)SOPRA_SYNTAX_HIGHLIGHTING} == (0|false|off|no) ]]; then
        editor_args+=(--disable-syntax-highlighting)
    fi
    if [[ ${(L)SOPRA_CURSOR_ANIMATION} == (0|false|off|no) ]]; then
        editor_args+=(--disable-cursor-animation)
    fi
    raw=$(
        SOPRA_GIT_STATUS_ITEM_FORMAT="$SOPRA_GIT_STATUS_ITEM_FORMAT" \
        SOPRA_GIT_STATUS_FORMAT="$SOPRA_GIT_STATUS_FORMAT" \
        SOPRA_GIT_STATUS_SEPARATOR="$SOPRA_GIT_STATUS_SEPARATOR" \
        SOPRA_WIDGET_STATUS="$_SOPRA_WIDGET_STATUS" \
        SOPRA_WIDGET_DURATION="$_SOPRA_WIDGET_DURATION" \
        SOPRA_WIDGET_USER="$USER" SOPRA_WIDGET_HOST="$HOST" \
        SOPRA_CLOCK_FORMAT="$SOPRA_CLOCK_FORMAT" \
        SOPRA_EXPANDED_TRANSIENT_PROMPT="$expanded_transient" \
        "$SOPRA_BIN" "${editor_args[@]}" </dev/tty
    ) || return 1
    _sopra_parse_result "$raw"
}

_sopra_prepare_transient() {
    _sopra_shell_prompt "$SOPRA_TRANSIENT_PROMPT"
    PROMPT=$REPLY
    RPROMPT=''
    # A pending resize can make ZLE redraw its cached full prompt on accept.
    zle .reset-prompt
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
            directory)
                if builtin cd -- "$_SOPRA_RESULT_DIRECTORY"; then
                    PROMPT=$SOPRA_PROMPT
                    RPROMPT=$SOPRA_RPROMPT
                fi
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
