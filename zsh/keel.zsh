if [[ -n ${_KEEL_ZSH_LOADED:-} ]]; then
    return 0
fi

typeset -g _KEEL_ZSH_LOADED=1
typeset -g _KEEL_ZSH_DIR=${${(%):-%N}:A:h}
source "$_KEEL_ZSH_DIR/keel-vars.zsh"
source "$_KEEL_ZSH_DIR/keel-cache.zsh"

if [[ ! -o interactive ]]; then
    return 0
fi

# The installed wrapper exports this path; ordinary stock Zsh sessions do not.
if [[ -z ${KEEL_MODULE_PATH:-} ]]; then
    return 0
fi

zmodload -i zsh/zle || return 1
zmodload -i zsh/parameter || return 1
zmodload -i zsh/datetime || true
autoload -Uz add-zle-hook-widget || return 1

if [[ -n ${KEEL_MODULE_PATH:-} ]]; then
    module_path=("$KEEL_MODULE_PATH" ${module_path:-})
fi

if ! zmodload -i keel; then
    print -u2 'keel: native module unavailable; use scripts/start-keel.sh'
    return 1
fi

source "$_KEEL_ZSH_DIR/keel-capture.zsh"
source "$_KEEL_ZSH_DIR/keel-completion.zsh"
source "$_KEEL_ZSH_DIR/keel-completion-response.zsh"
source "$_KEEL_ZSH_DIR/keel-widgets.zsh"
source "$_KEEL_ZSH_DIR/keel-lifecycle.zsh"

add-zle-hook-widget line-init keel-native-line-init
add-zle-hook-widget line-finish keel-native-line-finish
add-zle-hook-widget line-pre-redraw _keel_completion_pre_redraw
add-zle-hook-widget line-init _keel_completion_line_init
add-zle-hook-widget line-finish _keel_completion_line_finish
add-zle-hook-widget line-init _keel_cursor_line_init
add-zle-hook-widget line-finish _keel_cursor_line_finish
zle -N _keel_accept_widget
zle -N _keel_select_previous_widget
zle -N _keel_select_next_widget
zle -N _keel_dismiss_widget
zle -N _keel_accept_enter_widget
zle -N _keel_accept_linefeed_widget
zle -N _keel_completion_apply_widget
_KEEL_ZSH_HOOKS=1
_keel_bind_keys
