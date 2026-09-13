_keel_help_positional_kind() {
    emulate -L zsh

    local command_name=$1
    local positional_index=$2
    local field=$'\x1f'
    local -a positionals

    REPLY=''
    positionals=("${(@s:$field:)${_KEEL_HELP_POSITIONALS[$command_name]-}}")
    if (( positional_index > 0 && positional_index <= ${#positionals} )); then
        REPLY=${_KEEL_HELP_POSITIONAL_KINDS[${command_name}${field}${positionals[positional_index]}]-}
    fi
}

_keel_help_completion() {
    emulate -L zsh
    local command_name=${words[1]:t}
    local field=$'\x1f'
    local current=${words[CURRENT]-}
    local previous=${words[CURRENT-1]-}
    local key value positional_kind
    integer positional_index
    local -a options details positionals positional_details values

    options=("${(@s:$field:)${_KEEL_HELP_OPTIONS[$command_name]-}}")
    details=()
    for value in "${options[@]}"; do
        key="${command_name}${field}${value}"
        details+=("${_KEEL_HELP_OPTION_DETAILS[$key]-}")
    done

    key="${command_name}${field}${previous}"
    case ${_KEEL_HELP_OPTION_KINDS[$key]-} in
        directories)
            _directories
            return 0
            ;;
        files|path|both)
            _files
            return 0
            ;;
    esac
    if [[ -n ${_KEEL_HELP_OPTION_VALUES[$key]-} ]]; then
        values=("${(@s:$field:)${_KEEL_HELP_OPTION_VALUES[$key]}}")
        compadd -d "${(Oa)values}" -- "${values[@]}"
        return 0
    fi

    if [[ $current == -* ]]; then
        compadd -d details -- "${options[@]}"
        return 0
    fi

    positionals=("${(@s:$field:)${_KEEL_HELP_POSITIONALS[$command_name]-}}")
    positional_index=${CURRENT:-1}
    (( positional_index-- ))
    _keel_help_positional_kind "$command_name" "$positional_index"
    positional_kind=$REPLY
    case $positional_kind in
        directories)
            _directories
            return 0
            ;;
        files|path|both)
            _files
            return 0
            ;;
    esac
    positional_details=()
    for value in "${positionals[@]}"; do
        positional_details+=("${_KEEL_HELP_POSITIONAL_DETAILS[${command_name}${field}${value}]-}")
    done
    compadd -d positional_details -- "${positionals[@]}"
}

_keel_completion_load_help_provider() {
    emulate -L zsh
    local command_name=${1:t}

    _keel_help_load "$command_name" || return 1
    compdef _keel_help_completion "$command_name"
}
