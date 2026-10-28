_sopra_prompt_defaults() {
    if [[ -z ${_SOPRA_CONFIG_PROMPT+x} ]]; then
        typeset -g _SOPRA_CONFIG_PROMPT=$'[bold][red]{user}[/][/] [dim]in[/] [bold][cyan]{path}[/][/]\n[dim]$[/] '
    fi
    if [[ -z ${_SOPRA_CONFIG_RPROMPT+x} ]]; then
        typeset -g _SOPRA_CONFIG_RPROMPT='[if git][dim]on [/][bold][cyan]{git}[/][/][if git_status] {git_status}[/][/]'
    fi
    if [[ -z ${_SOPRA_CONFIG_GIT_STATUS_FORMAT+x} ]]; then
        typeset -g _SOPRA_CONFIG_GIT_STATUS_FORMAT='[dim][[[/]{items}[dim]][/]'
    fi
    if [[ -z ${_SOPRA_CONFIG_GIT_STATUS_SEPARATOR+x} ]]; then
        typeset -g _SOPRA_CONFIG_GIT_STATUS_SEPARATOR='[dim]|[/]'
    fi
    if [[ -z ${_SOPRA_CONFIG_GIT_STATUS_ITEM_FORMAT+x} ]]; then
        typeset -g _SOPRA_CONFIG_GIT_STATUS_ITEM_FORMAT='[bold][magenta]{item}[/][/]'
    fi
    if [[ -z ${_SOPRA_CONFIG_PATH_BEFORE+x} ]]; then
        typeset -g _SOPRA_CONFIG_PATH_BEFORE='{text}'
    fi
    if [[ -z ${_SOPRA_CONFIG_PATH_SELECTED+x} ]]; then
        typeset -g _SOPRA_CONFIG_PATH_SELECTED='[reverse]{text}[/]'
    fi
    if [[ -z ${_SOPRA_CONFIG_PATH_AFTER+x} ]]; then
        typeset -g _SOPRA_CONFIG_PATH_AFTER='[dim]{text}[/]'
    fi
    if [[ -z ${_SOPRA_CONFIG_CLOCK_FORMAT+x} ]]; then
        typeset -g _SOPRA_CONFIG_CLOCK_FORMAT='%H:%M:%S'
    fi
    if [[ -z ${_SOPRA_CONFIG_SECONDARY_PROMPT+x} ]]; then
        typeset -g _SOPRA_CONFIG_SECONDARY_PROMPT='> '
    fi
    if [[ -z ${_SOPRA_CONFIG_TRANSIENT_PROMPT+x} ]]; then
        typeset -g _SOPRA_CONFIG_TRANSIENT_PROMPT='$ '
    fi
}


_sopra_shell_prompt() {
    emulate -L zsh
    local remaining=$1 output='' tag code prefix
    local -a styles
    while [[ -n $remaining ]]; do
        if [[ $remaining == '[['* ]]; then
            output+='['
            remaining=${remaining[3,-1]}
        elif [[ $remaining =~ '^\[([a-z]+|/)\]' ]]; then
            tag=$match[1]
            _sopra_style_code "$tag"
            code=$REPLY
            if [[ -n $code ]]; then
                styles+=("$code")
                output+=$'%{\e['"$code"$'m%}'
            elif [[ $tag == / && ${#styles} -gt 0 ]]; then
                styles[-1]=()
                output+=$'%{\e[0m%}'
                for prefix in "${styles[@]}"; do
                    output+=$'%{\e['"$prefix"$'m%}'
                done
            else
                output+=$MATCH
            fi
            remaining=${remaining[$(( ${#MATCH} + 1 )),-1]}
        else
            output+=${remaining[1]}
            remaining=${remaining[2,-1]}
        fi
    done
    REPLY=$output
}

_sopra_style_code() {
    REPLY=''
    case $1 in
        dim) REPLY=2 ;; bold) REPLY=1 ;; reverse) REPLY=7 ;;
        italic) REPLY=3 ;; underline) REPLY=4 ;;
        default) REPLY=39 ;; black) REPLY=30 ;; red) REPLY=31 ;;
        green) REPLY=32 ;; yellow) REPLY=33 ;;
        blue) REPLY=34 ;; magenta) REPLY=35 ;;
        cyan) REPLY=36 ;; white) REPLY=37 ;;
    esac
}
