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
_sopra_load_prompt_config "$(_sopra_config_path)"
_sopra_load_prompt_config "${$(_sopra_config_path):h}/prompt.toml"
_sopra_load_settings
source "$script_dir/context.zsh"
source "$script_dir/prompt.zsh"
source "$script_dir/worker.zsh"
_sopra_prompt_defaults
if [[ -z ${_SOPRA_CONFIG_CURSOR_ANIMATION+x} ]]; then
    typeset -g _SOPRA_CONFIG_CURSOR_ANIMATION=1
fi
if [[ -z ${_SOPRA_CONFIG_SYNTAX_HIGHLIGHTING+x} ]]; then
    typeset -g _SOPRA_CONFIG_SYNTAX_HIGHLIGHTING=1
fi
if [[ -z ${_SOPRA_CONFIG_COMPLETION_ENABLED+x} ]]; then
    typeset -g _SOPRA_CONFIG_COMPLETION_ENABLED=1
fi

if [[ ! -x $SOPRA_BIN ]]; then
    print -u2 "sopra: executable not found at $SOPRA_BIN"
    return 1
fi

_sopra_prepare_transient() {
    _sopra_shell_prompt "${_sopra_transient_prompt-$_SOPRA_CONFIG_TRANSIENT_PROMPT}"
    PROMPT=$REPLY
    RPROMPT=''
    # A pending resize can make ZLE redraw its cached full prompt on accept.
    zle .reset-prompt
}

_sopra_clear_highlights() {
    region_highlight=()
}

_sopra_line_init() {
    local _sopra_transient_prompt=$_SOPRA_CONFIG_TRANSIENT_PROMPT
    _sopra_shell_prompt "$_SOPRA_CONFIG_SECONDARY_PROMPT"
    PS2=$REPLY
    if [[ $CONTEXT == cont ]]; then
        PROMPT=$_SOPRA_CONFIG_SECONDARY_PROMPT
        RPROMPT=''
        _sopra_transient_prompt=$_SOPRA_CONFIG_SECONDARY_PROMPT
    else
        PROMPT=$_SOPRA_CONFIG_PROMPT
        RPROMPT=$_SOPRA_CONFIG_RPROMPT
    fi
    _sopra_clear_highlights
    local -i arm_completion=0 suppress_completion=0
    while _sopra_edit "$arm_completion" "$suppress_completion"; do
        arm_completion=0
        suppress_completion=0
        BUFFER=$_SOPRA_RESULT_BUFFER
        CURSOR=$_SOPRA_RESULT_CURSOR
        _sopra_apply_result || return 0
    done
    zle -R
}

_sopra_apply_result() {
    case $_SOPRA_RESULT_ACTION in
        accept)
            _sopra_prepare_transient
            if [[ -n $_SOPRA_RESULT_HIGHLIGHTS ]]; then
                region_highlight=("${(@s.;.)_SOPRA_RESULT_HIGHLIGHTS}")
            fi
            zle .accept-line
            return 1
            ;;
        interrupt)
            _sopra_prepare_transient
            _sopra_clear_highlights
            BUFFER=''
            CURSOR=0
            zle .accept-line
            return 1
            ;;
        directory)
            if builtin cd -- "$_SOPRA_RESULT_DIRECTORY"; then
                PROMPT=$_SOPRA_CONFIG_PROMPT
                RPROMPT=$_SOPRA_CONFIG_RPROMPT
            fi
            ;;
        up|down) _sopra_history "$_SOPRA_RESULT_ACTION" ;;
        tab) zle expand-or-complete ;;
        eof) zle .eof; return 1 ;;
    esac
    return 0
}

_sopra_history() {
    local previous_buffer=$BUFFER previous_cursor=$CURSOR previous_history=${HISTNO:-}
    zle "$1-line-or-history"
    if [[ $BUFFER == $previous_buffer && $CURSOR == $previous_cursor \
        && ${HISTNO:-} == $previous_history ]]; then
        arm_completion=1
    else
        suppress_completion=1
    fi
}

_sopra_line_finish() {
    [[ $_SOPRA_RESULT_ACTION == accept ]] || return 0
    # Only multiline transcripts need display-only continuation prefixes.
    [[ $BUFFER == *$'\n'* ]] || return 0
    local original_buffer=$BUFFER
    local -i original_cursor=$CURSOR
    BUFFER=$_SOPRA_RESULT_DISPLAY
    local -a original_highlights=("${region_highlight[@]}")
    region_highlight=()
    if [[ $_SOPRA_RESULT_DISPLAY_HIGHLIGHTS != - ]]; then
        region_highlight=("${(@s.;.)_SOPRA_RESULT_DISPLAY_HIGHLIGHTS}")
    fi
    zle -I
    BUFFER=$original_buffer
    CURSOR=$original_cursor
    region_highlight=("${original_highlights[@]}")
}

zle -N zle-line-init _sopra_line_init
zle -N zle-line-finish _sopra_line_finish
