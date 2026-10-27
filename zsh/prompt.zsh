_sopra_prompt_defaults() {
    if [[ -z ${SOPRA_PROMPT+x} ]]; then
        typeset -g SOPRA_PROMPT='%F{cyan}{path}%f %# '
    fi
    if [[ -z ${SOPRA_RPROMPT+x} ]]; then
        typeset -g SOPRA_RPROMPT='%F{magenta}{git}%f'
    fi
    if [[ -z ${SOPRA_TRANSIENT_PROMPT+x} ]]; then
        typeset -g SOPRA_TRANSIENT_PROMPT='$ '
    fi
}

