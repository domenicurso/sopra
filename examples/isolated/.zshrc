PROMPT='%n in %~ $ '
RPROMPT=''
autoload -Uz compinit
compinit -u -d /dev/null
source "$SOPRA_REPO_ROOT/zsh/editor.zsh"
