if [[ ${KEEL_PROVIDER_MODE:-0} != 1 ]]; then
    print -u2 'keel: completion provider must run in provider mode'
    exit 2
fi

zmodload -i zsh/zle || exit 1
autoload -Uz compinit
if [[ -z ${_main_complete:-} && -z ${functions[_main_complete]:-} ]]; then
    compinit -u -d /dev/null >/dev/null 2>&1 || exit 1
fi
zstyle ':completion:*' menu no
zstyle ':completion:*' list no

_keel_provider_emit() {
    local label=$1 kind=$2 replacement=$3 detail=${4:-}
    [[ -n $label && -n $replacement ]] || return 0
    [[ $label != *$'\x1e'* && $label != *$'\x1f'* && $label != *$'\n'* ]] || return 0
    print -nu3 -- "$label"$'\x1f'"$kind"$'\x1f'"$replacement"$'\x1f'"$detail"$'\x1e' 2>/dev/null
}

_keel_provider_command_detail() {
    local candidate=$1
    if [[ -n ${commands[$candidate]-} ]]; then
        print -r -- "${commands[$candidate]}"
    elif [[ -n ${builtins[$candidate]-} ]]; then
        print -r -- 'shell builtin'
    fi
}

compadd() {
    local -a original=("$@") arguments values descriptions
    local prefix='' suffix='' display_prefix='' display_suffix=''
    local option value candidate label kind detail description_array
    integer index=1
    while (( index <= $#original )); do
        option=${original[index]}
        case $option in
            -p|-P)
                (( index++ ));
                if [[ $option == -p ]]; then
                    prefix=${original[index]-}
                else
                    display_prefix=${original[index]-}
                fi
                ;;
            -s|-S)
                (( index++ ));
                if [[ $option == -s ]]; then
                    suffix=${original[index]-}
                else
                    display_suffix=${original[index]-}
                fi
                ;;
            -d)
                (( index++ )); description_array=${original[index]-} ;;
        esac
        (( index++ ))
    done

    index=1
    while (( index <= $#original )); do
        option=${original[index]}
        case $option in
            -O|-A|-D|-d)
                (( index += 2 )); continue ;;
        esac
        arguments+=("$option")
        if [[ $option == -p || $option == -P || $option == -s || $option == -S ]]; then
            (( index++ )); arguments+=("${original[index]-}")
        fi
        (( index++ ))
    done

    builtin compadd -O values "${arguments[@]}"
    if [[ -n $description_array ]]; then
        descriptions=("${(@P)description_array}")
    fi
    integer item=1
    while (( item <= ${#values} )); do
        value=${values[item]}
        if [[ $value == -* ]]; then
            candidate=$value
            label=$value
        else
            candidate="$prefix$value$suffix"
            label="$display_prefix$value$display_suffix"
        fi
        kind=generic
        if [[ $candidate == */ || -d ${KEEL_PROVIDER_CWD:-.}/$candidate ]]; then
            kind=directory
            [[ $candidate == */ ]] || candidate+=/
        elif [[ -f ${KEEL_PROVIDER_CWD:-.}/$candidate ]]; then
            kind=file
        fi
        if [[ -z ${descriptions[item]-} && $candidate != */ ]]; then
            detail=$(_keel_provider_command_detail "$candidate")
        else
            detail=${descriptions[item]-}
        fi
        [[ $detail == "$label" ]] && detail=''
        _keel_provider_emit "$label" "$kind" "$candidate" "$detail"
        (( item++ ))
    done
}

_keel_provider_vared() {
    _normal -s
}

_keel_provider_capture() {
    BUFFER=$KEEL_PROVIDER_BUFFER
    CURSOR=${KEEL_PROVIDER_CURSOR:-0}
    zle complete-word >/dev/null 2>&1
    BUFFER=$KEEL_PROVIDER_BUFFER
    CURSOR=${KEEL_PROVIDER_CURSOR:-0}
}

zle -N _keel_provider_capture
for keymap in main emacs viins vicmd; do
    bindkey -M "$keymap" '^X' _keel_provider_capture 2>/dev/null || true
done
_comps[-vared-]=_keel_provider_vared
typeset -g _keel_provider_line=${KEEL_PROVIDER_BUFFER:-}
PROMPT=''
RPROMPT=''
vared -c _keel_provider_line >/dev/null 2>&1
