if [[ -n ${_KEEL_DEMO_LOADED:-} ]]; then
    return 0
fi
typeset -g _KEEL_DEMO_LOADED=1

if [[ ! -o interactive ]]; then
    return 0
fi

: "${KEEL_BIN:=${${(%):-%N}:A:h:h}/target/debug/keel-demo}"
: "${KEEL_PROMPT:=keel-demo ❯ }"

if [[ ! -x $KEEL_BIN ]]; then
    print -u2 "keel: demo binary not found at $KEEL_BIN"
    return 1
fi

# Stock Zsh owns the shell, prompt lifecycle, and command execution. This tiny
# widget is the only bridge: the Rust editor reads the existing tty, writes a
# result file, and hands the edited buffer back to ZLE.
_keel_demo_edit() {
    emulate -L zsh
    local prefix action buffer cursor
    prefix=$(mktemp "${TMPDIR:-/tmp}/keel-demo.XXXXXX") || return 1
    rm -f -- "$prefix"

    "$KEEL_BIN" \
        --buffer "$BUFFER" \
        --cursor "$CURSOR" \
        --prompt "$KEEL_PROMPT" \
        --result-prefix "$prefix" \
        </dev/tty >/dev/null

    if [[ ! -r $prefix.action || ! -r $prefix.buffer || ! -r $prefix.cursor ]]; then
        rm -f -- "$prefix.action" "$prefix.buffer" "$prefix.cursor"
        zle -R
        return 1
    fi

    action=$(<"$prefix.action")
    buffer=$(<"$prefix.buffer")
    cursor=$(<"$prefix.cursor")
    rm -f -- "$prefix.action" "$prefix.buffer" "$prefix.cursor"

    BUFFER=$buffer
    CURSOR=$cursor
    if [[ $action == accept || $action == interrupt ]]; then
        PROMPT=${KEEL_TRANSIENT_PROMPT:-❯ }
        if [[ $action == interrupt ]]; then
            # Rust already painted the aborted buffer as a transient line. Send the
            # break with an empty ZLE buffer so stock Zsh advances without erasing it.
            BUFFER=
            CURSOR=0
            zle .send-break
        else
            zle .accept-line
        fi
    else
        zle -R
    fi
}

zle -N _keel_demo_edit

_keel_demo_line_init() {
    PROMPT=$KEEL_PROMPT
    zle _keel_demo_edit
}

zle -N zle-line-init _keel_demo_line_init
