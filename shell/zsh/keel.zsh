# Native zsh-module activation for Keel.
#
# Build the module first:
#   /Users/dom/Projects/keel/scripts/build-zsh-module.sh
#
# Then source this file:
#   source /Users/dom/Projects/keel/shell/zsh/keel.zsh
#
# Optional:
#   KEEL_AUTO_START=0 source /Users/dom/Projects/keel/shell/zsh/keel.zsh
#   disables prompt-start auto-entry and keeps only the explicit widget bindings.

function keel-warn-frontend-limitations() {
  emulate -L zsh
  setopt localoptions no_aliases noshwordsplit
  local hook_count=0
  local special_widget_count=0
  local widget

  [[ -z "${KEEL_FRONTEND_WARNING_SHOWN:-}" ]] || return 0
  [[ "${KEEL_SUPPRESS_LIMITATION_WARNING:-0}" == "1" ]] && return 0

  hook_count=$(( ${#precmd_functions[@]} + ${#preexec_functions[@]} + ${#chpwd_functions[@]} ))
  for widget in \
    zle-line-init \
    zle-line-finish \
    zle-line-pre-redraw \
    zle-keymap-select \
    zle-isearch-update \
    zle-isearch-exit
  do
    zle -l "$widget" >/dev/null 2>&1 && (( special_widget_count += 1 ))
  done

  if (( hook_count > 0 || special_widget_count > 0 )); then
    print -u2 -- "keel: compatibility mode detected existing shell frontend integrations."
    print -u2 -- "keel: hooks=$hook_count special-widgets=$special_widget_count"
    print -u2 -- "keel: terminal-writing hooks or other zle frontend layers may still compose imperfectly."
  fi

  typeset -g KEEL_FRONTEND_WARNING_SHOWN=1
}

function keel-own-frontend() {
  emulate -L zsh
  setopt localoptions no_aliases noshwordsplit

  [[ "${KEEL_OWN_FRONTEND:-1}" == "1" ]] || return 0
  [[ -z "${KEEL_FRONTEND_OWNERSHIP_INSTALLED:-}" ]] || return 0

  typeset -g KEEL_SAVED_PROMPT="$PROMPT"
  typeset -g KEEL_SAVED_RPROMPT="$RPROMPT"
  typeset -g KEEL_SAVED_PROMPT2="${PS2:-}"
  typeset -g KEEL_SAVED_PROMPT4="${PS4:-}"
  typeset -ga KEEL_SAVED_PRECMD_FUNCTIONS=("${precmd_functions[@]}")
  typeset -ga KEEL_SAVED_PREEXEC_FUNCTIONS=("${preexec_functions[@]}")
  typeset -ga KEEL_SAVED_CHPWD_FUNCTIONS=("${chpwd_functions[@]}")
  typeset -ga KEEL_SAVED_PERIODIC_FUNCTIONS=("${periodic_functions[@]}")
  typeset -ga KEEL_SAVED_ZSHADDHISTORY_FUNCTIONS=("${zshaddhistory_functions[@]}")

  precmd_functions=()
  preexec_functions=()
  chpwd_functions=()
  periodic_functions=()
  zshaddhistory_functions=()

  local fn
  for fn in precmd preexec chpwd periodic zshaddhistory; do
    (( ${+functions[$fn]} )) || continue
    functions["keel--saved-$fn"]="${functions[$fn]}"
    unfunction "$fn"
  done

  local widget
  for widget in \
    zle-line-finish \
    zle-line-pre-redraw \
    zle-keymap-select \
    zle-isearch-update \
    zle-isearch-exit
  do
    if zle -l "$widget" >/dev/null 2>&1; then
      local saved_widget="keel--saved-$widget"
      { zle -D "$saved_widget" } >/dev/null 2>&1 || true
      zle -A "$widget" "$saved_widget"
      zle -D "$widget"
    fi
  done

  function precmd() {
    PROMPT=""
    RPROMPT=""
    PROMPT_EOL_MARK=""
    POSTEDIT=""
  }

  PROMPT=""
  RPROMPT=""
  PS2="> "
  PS4=""
  PROMPT_EOL_MARK=""
  POSTEDIT=""

  typeset -g KEEL_FRONTEND_OWNERSHIP_INSTALLED=1
}

function keel-shell-bin-path() {
  emulate -L zsh
  setopt localoptions no_aliases noshwordsplit
  local candidate

  if [[ -n "${KEEL_SHELL_BIN:-}" && -x "${KEEL_SHELL_BIN}" ]]; then
    print -r -- "${KEEL_SHELL_BIN}"
    return 0
  fi

  candidate="${KEEL_REPO_ROOT}/target/debug/keel-shell"
  if [[ -x "$candidate" ]]; then
    print -r -- "$candidate"
    return 0
  fi

  if (( ${+commands[keel-shell]} )); then
    print -r -- "${commands[keel-shell]}"
    return 0
  fi

  return 1
}

function keel-read-frontend-command() {
  emulate -L zsh
  setopt localoptions no_aliases noshwordsplit
  local shell_bin
  local accepted

  shell_bin="$(keel-shell-bin-path)" || {
    print -u2 -- "keel: missing keel-shell frontend binary"
    print -u2 -- "keel: build it with: $KEEL_REPO_ROOT/scripts/build-zsh-module.sh"
    return 127
  }

  accepted="$("$shell_bin" \
    --prompt "${KEEL_PROMPT_TEXT:-keel> }" \
    --buffer "$BUFFER" \
    --cursor "${CURSOR:-0}")"
  REPLY="$accepted"
}

function keel-session-loop() {
  local accepted
  local exit_status

  while true; do
    keel-read-frontend-command
    exit_status=$?

    case $exit_status in
      0)
        accepted="$REPLY"
        print -rn -- $'\r\n'
        if [[ -n "$accepted" ]]; then
          eval "$accepted" </dev/tty >/dev/tty 2>/dev/tty
        fi
        ;;
      130)
        ;;
      *)
        return $exit_status
        ;;
    esac
  done
}

function keel-edit-line-widget() {
  emulate -L zsh
  setopt localoptions no_aliases noshwordsplit
  local exit_status

  typeset -g KEEL_FRONTEND_SUBMITTED=0
  keel-read-frontend-command
  exit_status=$?

  case $exit_status in
    0)
      BUFFER="$REPLY"
      CURSOR=${#BUFFER}
      typeset -g KEEL_FRONTEND_SUBMITTED=1
      zle redisplay
      return 0
      ;;
    130)
      zle redisplay
      return 0
      ;;
    *)
      zle redisplay
      return $exit_status
      ;;
  esac
}

function keel-accept-line-widget() {
  emulate -L zsh
  setopt localoptions no_aliases noshwordsplit
  local exit_status
  local transient_prompt="${KEEL_TRANSIENT_PROMPT_TEXT:-}"

  [[ -n "$transient_prompt" ]] || transient_prompt="> "

  keel-read-frontend-command
  exit_status=$?

  case $exit_status in
    0)
      BUFFER="$REPLY"
      CURSOR=${#BUFFER}
      POSTEDIT="${transient_prompt}${BUFFER}"$'\n'
      zle .accept-line
      return 0
      ;;
    130)
      BUFFER=""
      CURSOR=0
      POSTEDIT=""
      zle redisplay
      return 0
      ;;
    *)
      zle redisplay
      return $exit_status
      ;;
  esac
}

function keel-line-init-widget() {
  emulate -L zsh
  setopt localoptions no_aliases noshwordsplit

  if [[ "${KEEL_OWN_FRONTEND:-1}" != "1" && -n "${KEEL_ORIGINAL_LINE_INIT_WIDGET:-}" ]]; then
    zle "$KEEL_ORIGINAL_LINE_INIT_WIDGET"
  fi

  if [[ -n "${KEEL_LINE_INIT_GUARD:-}" ]]; then
    return 0
  fi

  KEEL_LINE_INIT_GUARD=1
  zle keel-edit-line
  unset KEEL_LINE_INIT_GUARD

  if [[ "${KEEL_FRONTEND_SUBMITTED:-0}" == "1" ]]; then
    local transient_prompt="${KEEL_TRANSIENT_PROMPT_TEXT:-}"
    [[ -n "$transient_prompt" ]] || transient_prompt="> "
    POSTEDIT="${transient_prompt}${BUFFER}"$'\n'
    unset KEEL_FRONTEND_SUBMITTED
    zle .accept-line
  fi
}

typeset -g KEEL_REPO_ROOT="${KEEL_REPO_ROOT:-$HOME/Projects/keel}"
typeset -g KEEL_MODULE_PATH="${KEEL_MODULE_PATH:-$KEEL_REPO_ROOT/target/zsh/keel.so}"
typeset -g KEEL_MODULE_DIR="${KEEL_MODULE_DIR:-${KEEL_MODULE_PATH:h}}"
typeset -g KEEL_MODULE_NAME="${KEEL_MODULE_NAME:-${${KEEL_MODULE_PATH:t}%.so}}"
typeset -g KEEL_PROMPT_TEXT="${KEEL_PROMPT_TEXT:-keel> }"
typeset -g KEEL_TRANSIENT_PROMPT_TEXT="${KEEL_TRANSIENT_PROMPT_TEXT:-> }"
typeset -g KEEL_AUTO_START="${KEEL_AUTO_START:-1}"
typeset -g KEEL_OWN_FRONTEND="${KEEL_OWN_FRONTEND:-1}"

if [[ ! -f "$KEEL_MODULE_PATH" ]]; then
  print -u2 -- "keel: missing module at $KEEL_MODULE_PATH"
  print -u2 -- "keel: build it with: $KEEL_REPO_ROOT/scripts/build-zsh-module.sh"
  return 1
fi

zmodload zsh/zle
{ zle -D keel-accept-line } >/dev/null 2>&1 || true
{ zle -D keel-edit-line } >/dev/null 2>&1 || true
module_path=("$KEEL_MODULE_DIR" $module_path)
zmodload "$KEEL_MODULE_NAME"
keel-activate
keel-own-frontend
keel-warn-frontend-limitations
zle -N keel-accept-line keel-accept-line-widget
zle -N keel-edit-line keel-edit-line-widget

if [[ "$KEEL_AUTO_START" == "1" && -z "${KEEL_LINE_INIT_INSTALLED:-}" ]]; then
  if zle -l zle-line-init >/dev/null 2>&1; then
    typeset -g KEEL_ORIGINAL_LINE_INIT_WIDGET="keel--orig-line-init"
    { zle -D "$KEEL_ORIGINAL_LINE_INIT_WIDGET" } >/dev/null 2>&1 || true
    zle -A zle-line-init "$KEEL_ORIGINAL_LINE_INIT_WIDGET"
  else
    typeset -g KEEL_ORIGINAL_LINE_INIT_WIDGET=""
  fi

  zle -N zle-line-init keel-line-init-widget
  typeset -g KEEL_LINE_INIT_INSTALLED=1
fi

if [[ "${KEEL_WRAP_ACCEPT_LINE:-1}" == "1" ]]; then
  bindkey '^M' keel-accept-line
  bindkey '^J' keel-accept-line
fi

if [[ -n "${KEEL_BIND_KEY:-}" ]]; then
  bindkey "$KEEL_BIND_KEY" keel-edit-line
else
  bindkey '^X^K' keel-edit-line
fi

zle reset-prompt >/dev/null 2>&1 || true
