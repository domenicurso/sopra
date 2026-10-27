zmodload zsh/datetime

typeset -gF _SOPRA_COMMAND_STARTED=0
typeset -g _SOPRA_WIDGET_STATUS=0 _SOPRA_WIDGET_DURATION=''

_sopra_command_started() {
    _SOPRA_COMMAND_STARTED=$EPOCHREALTIME
}

_sopra_command_finished() {
    local -i previous_status=$?
    typeset -g _SOPRA_WIDGET_STATUS=$previous_status
    if (( _SOPRA_COMMAND_STARTED > 0 )); then
        typeset -gF 3 _SOPRA_WIDGET_DURATION=$(( EPOCHREALTIME - _SOPRA_COMMAND_STARTED ))
        _SOPRA_COMMAND_STARTED=0
    fi
    return 0
}

typeset -ga precmd_functions preexec_functions
precmd_functions=(_sopra_command_finished ${precmd_functions:#_sopra_command_finished})
preexec_functions=(_sopra_command_started ${preexec_functions:#_sopra_command_started})
