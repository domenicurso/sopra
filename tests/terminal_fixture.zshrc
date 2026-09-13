PROMPT='native-prompt> '
RPROMPT=''
autoload -Uz compinit
compinit -C

typeset -a _keel_screen_values
typeset -a _keel_screen_descriptions
_keel_screen_values=(item-00 item-01 item-02 item-03 item-04 item-05 item-06 item-07 item-08 item-09 item-10 item-11 item-12 item-13 item-14 item-15 item-16 item-17 item-18 item-19)
_keel_screen_descriptions=('item zero' 'item one' 'item two' 'item three' 'item four' 'item five' 'item six' 'item seven' 'item eight' 'item nine' 'item ten' 'item eleven' 'item twelve' 'item thirteen' 'item fourteen' 'item fifteen' 'item sixteen' 'item seventeen' 'item eighteen' 'item nineteen')
_keel_screen_complete() {
    compadd -d _keel_screen_descriptions -- "${_keel_screen_values[@]}"
}
keel-screen() {
    print -r -- "keel-screen:$*"
}
compdef _keel_screen_complete keel-screen

source "$KEEL_REPO/zsh/keel.zsh"
