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
        typeset -gF 6 _SOPRA_WIDGET_DURATION=$(( EPOCHREALTIME - _SOPRA_COMMAND_STARTED ))
        _SOPRA_COMMAND_STARTED=0
    fi
    return 0
}

typeset -ga precmd_functions preexec_functions
precmd_functions=(_sopra_command_finished ${precmd_functions:#_sopra_command_finished})
preexec_functions=(_sopra_command_started ${preexec_functions:#_sopra_command_started})

_sopra_shell_context() {
    local -a alias_list
    local alias_name
    for alias_name in ${(k)aliases}; do
        [[ $alias_name == sopra ]] || alias_list+=($alias_name)
    done
    printf -v SOPRA_ALIASES '%s\n' "${alias_list[@]}"
    local -a function_list
    local function_name
    for function_name in ${(k)functions}; do
        [[ $function_name == sopra ]] || function_list+=($function_name)
    done
    printf -v SOPRA_FUNCTIONS '%s\n' "${function_list[@]}"
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
        [[ $command_name == sopra ]] && continue
        command_list+=("$command_name"$'\t'"${commands[$command_name]}")
    done
    command_list+=("sopra"$'\t'"$SOPRA_BIN")
    printf -v SOPRA_COMMANDS '%s\n' "${command_list[@]}"
}
