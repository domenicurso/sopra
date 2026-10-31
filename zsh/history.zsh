# Keep Zsh as the history owner so native history commands and Sopra agree.
if [[ -z ${HISTFILE:-} ]]; then
    HISTFILE=${ZDOTDIR:-$HOME}/.zsh_history
    (( SAVEHIST > 0 )) || SAVEHIST=10000
    (( HISTSIZE > SAVEHIST )) || HISTSIZE=$(( SAVEHIST + 1000 ))
fi

setopt APPEND_HISTORY EXTENDED_HISTORY
# SHARE_HISTORY already appends and imports entries. Otherwise write after
# execution so timestamps include completed commands, without waiting for exit.
if [[ ! -o SHARE_HISTORY && ! -o INC_APPEND_HISTORY && ! -o INC_APPEND_HISTORY_TIME ]]; then
    setopt INC_APPEND_HISTORY_TIME
fi
