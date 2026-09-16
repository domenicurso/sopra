PROMPT='%n in %~ ❯ '
RPROMPT=''
autoload -Uz compinit
compinit -u -d /dev/null
source "$KEEL_REPO_ROOT/zsh/keel.zsh"
