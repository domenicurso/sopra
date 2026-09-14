typeset -gA _KEEL_HELP_OPTIONS
typeset -gA _KEEL_HELP_OPTION_DETAILS
typeset -gA _KEEL_HELP_POSITIONALS
typeset -gA _KEEL_HELP_POSITIONAL_DETAILS
typeset -gA _KEEL_HELP_POSITIONAL_KINDS
typeset -gA _KEEL_HELP_OPTION_KINDS
typeset -gA _KEEL_HELP_OPTION_VALUES
typeset -gA _KEEL_HELP_HELP_LOADED

_keel_help_trim() {
    REPLY=$1
    while [[ -n $REPLY && $REPLY == [[:space:]]* ]]; do
        REPLY=${REPLY[2,-1]}
    done
    while [[ -n $REPLY && $REPLY == *[[:space:]] ]]; do
        REPLY=${REPLY[1,-2]}
    done
}

_keel_help_append() {
    local command_name=$1
    local value=$2
    local detail=$3
    local field=$'\x1f'
    local key="${command_name}${field}${value}"

    [[ -n $value ]] || return 0
    [[ -n ${_KEEL_HELP_OPTION_DETAILS[$key]-} ]] && return 0
    _KEEL_HELP_OPTIONS[$command_name]+="${_KEEL_HELP_OPTIONS[$command_name]:+$field}$value"
    _KEEL_HELP_OPTION_DETAILS[$key]=$detail
}

_keel_help_append_positional() {
    local command_name=$1
    local value=$2
    local detail=$3
    local kind=${4:-}
    local field=$'\x1f'

    [[ -n $value ]] || return 0
    if [[ -n ${_KEEL_HELP_POSITIONAL_DETAILS[${command_name}${field}${value}]-} ]]; then
        return 0
    fi
    _KEEL_HELP_POSITIONALS[$command_name]+="${_KEEL_HELP_POSITIONALS[$command_name]:+$field}$value"
    _KEEL_HELP_POSITIONAL_DETAILS[${command_name}${field}${value}]=$detail
    [[ -n $kind ]] && _KEEL_HELP_POSITIONAL_KINDS[${command_name}${field}${value}]=$kind
}

_keel_help_append_command() {
    local command_name=$1
    local value=$2
    local detail=$3
    local field=$'\x1f'
    local key="${command_name}${field}${value}"

    [[ -n $value ]] || return 0
    [[ -n ${_KEEL_HELP_COMMAND_DETAILS[$key]-} ]] && return 0
    _KEEL_HELP_COMMANDS[$command_name]+="${_KEEL_HELP_COMMANDS[$command_name]:+$field}$value"
    _KEEL_HELP_COMMAND_DETAILS[$key]=$detail
}

_keel_help_append_values() {
    local command_name=$1
    local option=$2
    local list=$3
    local detail=$4
    local field=$'\x1f'
    local value key
    local -a values

    list=${list%%.*}
    list=${list//\'/}
    list=${list//\"/}
    values=("${(@s:,:)list}")
    for value in "${values[@]}"; do
        _keel_help_trim "$value"
        value=$REPLY
        value=${value%% \(*}
        _keel_help_trim "$value"
        [[ -n $value ]] || continue
        if [[ -n $option ]]; then
            key="${command_name}${field}${option}"
            _KEEL_HELP_OPTION_VALUES[$key]+="${_KEEL_HELP_OPTION_VALUES[$key]:+$field}$value"
            _KEEL_HELP_OPTION_KINDS[$key]=values
        else
            _keel_help_append_positional "$command_name" "$value" "$detail"
        fi
    done
}

_keel_help_path_kind() {
    emulate -L zsh

    local spec=${(L)1}
    local detail_lower=${(L)2}
    local role=${spec#--}
    role=${role#-}
    role=${role%%[=<]*}

    REPLY=''
    if [[ $detail_lower == *file* &&
          ( $detail_lower == *directory* || $detail_lower == *folder* ) ]]; then
        REPLY=both
        return 0
    fi
    case $role in
        project|project-dir|project-directory|workspace|worktree|repository|repo|directory|dir|folder|cwd|root)
            REPLY=directories
            return 0
            ;;
    esac
    if [[ $detail_lower == *directory* || $detail_lower == *folder* ||
          $detail_lower == *' working directory'* || $detail_lower == *' working tree'* ||
          $detail_lower == *' dir'* ]]; then
        REPLY=directories
    elif [[ $detail_lower == *path* ]]; then
        REPLY=both
    elif [[ $detail_lower == *file* ]]; then
        REPLY=files
    fi
}

_keel_help_parse_line() {
    local command_name=$1
    local section=$2
    local line=$3
    local trimmed spec detail token option list path_kind
    local -a spec_words

    _keel_help_trim "$line"
    trimmed=$REPLY
    [[ -n $trimmed ]] || return 0

    if [[ $section == flags && $trimmed == -* ]]; then
        spec=$trimmed
        detail=$trimmed
        if [[ $trimmed == *'  '* ]]; then
            spec=${trimmed%%  *}
            detail=${trimmed#"$spec"}
            _keel_help_trim "$detail"
            detail=$REPLY
        fi
        spec_words=("${(@z)spec}")
        for token in "${spec_words[@]}"; do
            token=${token%,}
            [[ $token == -* ]] || continue
            option=${token%%[=<]*}
            [[ $option == -* ]] || continue
            _keel_help_append "$command_name" "$option" "$detail"
            _keel_help_path_kind "$option" "$detail"
            path_kind=$REPLY
            if [[ -z $path_kind &&
                  ( $spec == *'<'* || $spec == *'='* ) ]]; then
                path_kind=files
            fi
            if [[ -n $path_kind ]]; then
                _KEEL_HELP_OPTION_KINDS[${command_name}$'\x1f'$option]=$path_kind
            fi
            if [[ $detail == *'Accepted values'*:* ]]; then
                list=${detail#*:}
                _keel_help_append_values "$command_name" "$option" "$list" "$detail"
            elif [[ $detail == *'Valid options:'* ]]; then
                list=${detail#*Valid options:}
                _keel_help_append_values "$command_name" "$option" "$list" "$detail"
            fi
        done
        return 0
    fi

    if [[ $section == positionals ]]; then
        spec=${trimmed%%  *}
        detail=${trimmed#"$spec"}
        [[ $spec != "$trimmed" ]] || return 0
        _keel_help_trim "$detail"
        detail=$REPLY
        _keel_help_path_kind "$spec" "$detail"
        path_kind=$REPLY
        if [[ $detail == *'Accepted values'*:* ]]; then
            list=${detail#*:}
            _keel_help_append_values "$command_name" '' "$list" "$detail"
        elif [[ $detail == *'Valid options:'* ]]; then
            list=${detail#*Valid options:}
            _keel_help_append_values "$command_name" '' "$list" "$detail"
        fi
        _keel_help_append_positional "$command_name" "$spec" "$detail" "$path_kind"
        return 0
    fi

    if [[ $section == commands ]]; then
        spec=${trimmed%%  *}
        detail=${trimmed#"$spec"}
        [[ $spec != "$trimmed" ]] || return 0
        _keel_help_trim "$detail"
        _keel_help_append_command "$command_name" "$spec" "$REPLY"
    fi
}

_keel_help_load() {
    emulate -L zsh
    local command_name=${1:t}
    local help_output line section=''
    local -a lines

    if (( $+_KEEL_HELP_HELP_LOADED[$command_name] )); then
        [[ -n ${_KEEL_HELP_OPTIONS[$command_name]-} ||
           -n ${_KEEL_HELP_POSITIONALS[$command_name]-} ||
           -n ${_KEEL_HELP_COMMANDS[$command_name]-} ]]
        return
    fi
    if (( $+functions[_keel_completion_prefetch_poll] )); then
        _keel_completion_prefetch_poll "$command_name"
    fi
    _KEEL_HELP_HELP_LOADED[$command_name]=1
    if (( ${_KEEL_COMPLETION_PREFETCH_HELP_READY[$command_name]:-0} )); then
        help_output=${_KEEL_COMPLETION_PREFETCH_HELP[$command_name]}
    elif (( ${_KEEL_COMPLETION_CAPTURE:-0} )); then
        help_output=$("$command_name" --help 2>&1) || true
    else
        # The line-pre-redraw hook must never execute a CLI synchronously. The
        # completion worker can load help after it forks, while the parent
        # keeps accepting keystrokes and polling the prefetch result.
        return 1
    fi
    [[ -n $help_output ]] || return 1
    lines=("${(@f)help_output}")
    for line in "${lines[@]}"; do
        _keel_help_trim "$line"
        line=$REPLY
        case $line in
            Flags:*|Options:*|Arguments:*) section=flags; continue ;;
            'Positionals:'*|'Positional Variables:'*|'Positional Arguments:'*) section=positionals; continue ;;
            Commands:*|Subcommands:*) section=commands; continue ;;
            Usage:*|Description:*|Examples:*) section=''; continue ;;
        esac
        _keel_help_parse_line "$command_name" "$section" "$line"
    done
    [[ -n ${_KEEL_HELP_OPTIONS[$command_name]-} ||
       -n ${_KEEL_HELP_POSITIONALS[$command_name]-} ||
       -n ${_KEEL_HELP_COMMANDS[$command_name]-} ]]
}
