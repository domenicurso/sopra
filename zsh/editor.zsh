if [[ -n ${_SOPRA_LOADED:-} ]]; then
    return 0
fi
typeset -g _SOPRA_LOADED=1
unset _SOPRA_SERVER_SOCKET

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
source "$script_dir/protocol.zsh"
source "$script_dir/context.zsh"
source "$script_dir/history.zsh"
source "$script_dir/worker.zsh"
source "$script_dir/session.zsh"
if [[ ! -x $SOPRA_BIN ]]; then
    print -u2 "sopra: executable not found at $SOPRA_BIN"
    return 1
fi

sopra() {
    emulate -L zsh
    "$SOPRA_BIN" "$@"
}

_sopra_prepare_transient() {
    PROMPT=$_SOPRA_RESULT_PROMPT
    PS2=$_SOPRA_RESULT_SECONDARY
    RPROMPT=''
    # A pending resize can make ZLE redraw its cached full prompt on accept.
    zle .reset-prompt
}

_sopra_clear_highlights() {
    region_highlight=()
}

_sopra_line_init() {
    _sopra_server_start || return 1
    PROMPT=''
    RPROMPT=''
    _sopra_clear_highlights
    while _sopra_edit; do
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
                PROMPT=''
                RPROMPT=''
            fi
            ;;
        tab) zle expand-or-complete ;;
        eof)
            # EOF is handled by ZLE's input loop rather than an invokable widget.
            zle -U $'\x04'
            return 1
            ;;
    esac
    return 0
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
_sopra_hide_native_prompt() { PROMPT=''; RPROMPT=''; }
_sopra_hide_native_prompt
precmd_functions=(${precmd_functions:#_sopra_hide_native_prompt} _sopra_hide_native_prompt)
