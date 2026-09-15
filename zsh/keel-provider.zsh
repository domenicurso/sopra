if [[ ${KEEL_PROVIDER_MODE:-0} != 1 ]]; then
    print -u2 'keel: completion provider must run in provider mode'
    exit 2
fi

zmodload -i zsh/zle || exit 1
autoload -Uz compinit
if [[ -z ${_main_complete:-} && -z ${functions[_main_complete]:-} ]]; then
    compinit -u -d /dev/null >/dev/null 2>&1 || exit 1
fi

_keel_provider_emit() {
    local label=$1 kind=$2 replacement=$3 detail=${4:-}
    [[ -n $label && -n $replacement ]] || return 0
    [[ $label != *$'\x1e'* && $label != *$'\x1f'* && $label != *$'\n'* ]] || return 0
    print -nu3 -- "$label"$'\x1f'"$kind"$'\x1f'"$replacement"$'\x1f'"$detail"$'\x1e' 2>/dev/null
}

compadd() {
    local -a original=("$@") arguments values descriptions
    local prefix='' suffix='' option value candidate kind
    integer index=1
    while (( index <= $#original )); do
        option=${original[index]}
        case $option in
            -p|-P)
                (( index++ )); prefix=${original[index]-} ;;
            -s|-S)
                (( index++ )); suffix=${original[index]-} ;;
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
        arguments+=($option)
        if [[ $option == -p || $option == -P || $option == -s || $option == -S ]]; then
            (( index++ )); arguments+=(${original[index]-})
        fi
        (( index++ ))
    done

    builtin compadd -O values -D descriptions "${arguments[@]}" || return 0
    integer item=1
    for item in {1..${#values}}; do
        value=${values[item]}
        candidate="$prefix$value$suffix"
        kind=generic
        if [[ $candidate == */ || -d ${KEEL_PROVIDER_CWD:-.}/$candidate ]]; then
            kind=directory
            [[ $candidate == */ ]] || candidate+=/
        elif [[ -f ${KEEL_PROVIDER_CWD:-.}/$candidate ]]; then
            kind=file
        fi
        _keel_provider_emit "$candidate" "$kind" "$candidate" "${descriptions[item]-}"
    done
}

_keel_provider_capture() {
    BUFFER=$KEEL_PROVIDER_BUFFER
    CURSOR=${KEEL_PROVIDER_CURSOR:-0}
    zle complete-word >/dev/null 2>&1
    BUFFER=$KEEL_PROVIDER_BUFFER
    CURSOR=${KEEL_PROVIDER_CURSOR:-0}
    zle .accept-line
}

zle -N _keel_provider_capture
bindkey -M main '^X' _keel_provider_capture
typeset -g _keel_provider_line=${KEEL_PROVIDER_BUFFER:-}
PROMPT=''
RPROMPT=''
vared -c _keel_provider_line >/dev/null 2>&1
