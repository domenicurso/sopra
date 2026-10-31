PROMPT='%n in %~ $ '
RPROMPT=''
HISTFILE=/dev/null
SAVEHIST=0
autoload -Uz compinit
compinit -u -d /dev/null
source "$SOPRA_REPO_ROOT/zsh/editor.zsh"
