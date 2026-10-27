typeset -g _SOPRA_RESULT_ACTION=''
typeset -g _SOPRA_RESULT_BUFFER=''
typeset -g _SOPRA_RESULT_CURSOR=0
typeset -g _SOPRA_RESULT_HIGHLIGHTS=''
typeset -g _SOPRA_RESULT_DIRECTORY=''

_sopra_hex_decode() {
    emulate -L zsh
    local hex=$1 escaped='' pair
    [[ $hex == - ]] && hex=''
    (( ${#hex} % 2 == 0 )) || return 1
    while [[ -n $hex ]]; do
        pair=${hex[1,2]}
        [[ $pair == [[:xdigit:]][[:xdigit:]] ]] || return 1
        escaped+="\\x$pair"
        hex=${hex[3,-1]}
    done
    typeset -g _SOPRA_RESULT_BUFFER
    printf -v _SOPRA_RESULT_BUFFER '%b' "$escaped"
}

_sopra_parse_result() {
    emulate -L zsh
    local raw=$1
    local -a fields
    IFS=$'\t' read -rA fields <<< "$raw"
    [[ ${fields[1]-} == K1 && ${fields[2]-} == (accept|interrupt|up|down|tab|eof|directory) ]] || return 1
    [[ ${fields[3]-} == <-> ]] || return 1
    _sopra_hex_decode "${fields[6]-}" || return 1
    typeset -g _SOPRA_RESULT_DIRECTORY=$_SOPRA_RESULT_BUFFER
    _sopra_hex_decode "${fields[4]-}" || return 1
    typeset -g _SOPRA_RESULT_ACTION=${fields[2]}
    typeset -g _SOPRA_RESULT_CURSOR=${fields[3]}
    typeset -g _SOPRA_RESULT_HIGHLIGHTS=${fields[5]-}
    [[ $_SOPRA_RESULT_HIGHLIGHTS == - ]] && _SOPRA_RESULT_HIGHLIGHTS=''
    return 0
}

