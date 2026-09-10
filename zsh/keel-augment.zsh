#!/usr/bin/env zsh

if [[ -n "${_KEEL_AUGMENT_LOADED:-}" ]]; then
  return 0
fi

typeset -g _KEEL_AUGMENT_LOADED=1
typeset -g _KEEL_AUGMENT_ROOT="${KEEL_AUGMENT_ROOT:-${${(%):-%N}:A:h:h}}"
typeset -g _KEEL_AUGMENT_BINARY="${KEEL_AUGMENT_BINARY:-$_KEEL_AUGMENT_ROOT/target/debug/keel-augment}"
typeset -g _KEEL_AUGMENT_ACTIVE=0
typeset -g _KEEL_AUGMENT_BASE_RPROMPT=''
typeset -g _KEEL_AUGMENT_BASE_RPROMPT_INDENT=1
typeset -g _KEEL_AUGMENT_SKIP_PRE_REDRAW=0
typeset -g _KEEL_AUGMENT_NEW_LINE=1
typeset -ga _KEEL_AUGMENT_SAVED_WIDGETS

function _keel-augment-delegate() {
  local original="$1"
  shift
  _KEEL_AUGMENT_NEW_LINE=0
  zle "$original" "$@"
  _keel-augment-render-current
  _KEEL_AUGMENT_SKIP_PRE_REDRAW=1
  zle reset-prompt
  _KEEL_AUGMENT_SKIP_PRE_REDRAW=0
}

function _keel-augment-send-break() {
  _KEEL_AUGMENT_NEW_LINE=1
  _keel-augment-render-empty
  zle .keel-augment-original-send-break "$@"
}

function _keel-augment-self-insert() {
  _keel-augment-delegate .keel-augment-original-self-insert "$@"
}

function _keel-augment-backward-delete-char() {
  _keel-augment-delegate .keel-augment-original-backward-delete-char "$@"
}

function _keel-augment-delete-char() {
  _keel-augment-delegate .keel-augment-original-delete-char "$@"
}

function _keel-augment-backward-char() {
  _keel-augment-delegate .keel-augment-original-backward-char "$@"
}

function _keel-augment-forward-char() {
  _keel-augment-delegate .keel-augment-original-forward-char "$@"
}

function _keel-augment-beginning-of-line() {
  _keel-augment-delegate .keel-augment-original-beginning-of-line "$@"
}

function _keel-augment-end-of-line() {
  _keel-augment-delegate .keel-augment-original-end-of-line "$@"
}

function _keel-augment-up-line-or-history() {
  _keel-augment-delegate .keel-augment-original-up-line-or-history "$@"
}

function _keel-augment-down-line-or-history() {
  _keel-augment-delegate .keel-augment-original-down-line-or-history "$@"
}

function _keel-augment-pre-redraw() {
  (( _KEEL_AUGMENT_ACTIVE )) || return 0
  (( _KEEL_AUGMENT_SKIP_PRE_REDRAW )) && return 0
  (( _KEEL_AUGMENT_NEW_LINE )) && return 0

  local badge
  badge="$($_KEEL_AUGMENT_BINARY \
    --buffer "$BUFFER" \
    --cursor "$CURSOR" \
    --width "${COLUMNS:-80}" \
    --keymap "${KEYMAP:-main}" 2>/dev/null)" || {
    RPROMPT="$_KEEL_AUGMENT_BASE_RPROMPT"
    return 0
  }

  RPROMPT="${_KEEL_AUGMENT_BASE_RPROMPT}${badge}"
  return 0
}

function _keel-augment-render-current() {
  local badge
  badge="$($_KEEL_AUGMENT_BINARY \
    --buffer "${BUFFER:-}" \
    --cursor "${CURSOR:-0}" \
    --width "${COLUMNS:-80}" \
    --keymap "${KEYMAP:-main}" 2>/dev/null)" || return 0
  RPROMPT="${_KEEL_AUGMENT_BASE_RPROMPT}${badge}"
}

function _keel-augment-render-empty() {
  local badge
  badge="$($_KEEL_AUGMENT_BINARY \
    --buffer "" \
    --cursor 0 \
    --width "${COLUMNS:-80}" \
    --keymap "${KEYMAP:-main}" 2>/dev/null)" || return 0
  RPROMPT="${_KEEL_AUGMENT_BASE_RPROMPT}${badge}"
}

function _keel-augment-line-finish() {
  (( _KEEL_AUGMENT_ACTIVE )) || return 0
  _KEEL_AUGMENT_NEW_LINE=1
  _keel-augment-render-empty
  return 0
}

function keel-augment-enable() {
  (( _KEEL_AUGMENT_ACTIVE )) && return 0

  if [[ ! -x "$_KEEL_AUGMENT_BINARY" ]]; then
    print -u2 "keel-augment: build $_KEEL_AUGMENT_BINARY first"
    return 1
  fi

  autoload -Uz add-zle-hook-widget
  _KEEL_AUGMENT_BASE_RPROMPT="${RPROMPT-}"
  _KEEL_AUGMENT_BASE_RPROMPT_INDENT="${ZLE_RPROMPT_INDENT:-1}"
  ZLE_RPROMPT_INDENT=0
  _KEEL_AUGMENT_SAVED_WIDGETS=()

  local widget
  for widget in \
    self-insert \
    backward-delete-char \
    delete-char \
    backward-char \
    forward-char \
    beginning-of-line \
    end-of-line \
    up-line-or-history \
    down-line-or-history; do
    if zle -A "$widget" ".keel-augment-original-$widget" 2>/dev/null; then
      zle -N "$widget" "_keel-augment-$widget"
      _KEEL_AUGMENT_SAVED_WIDGETS+=("$widget")
    fi
  done

  if zle -A send-break .keel-augment-original-send-break 2>/dev/null; then
    zle -N send-break _keel-augment-send-break
    _KEEL_AUGMENT_SAVED_WIDGETS+=(send-break)
  fi

  add-zle-hook-widget line-pre-redraw _keel-augment-pre-redraw
  add-zle-hook-widget line-init _keel-augment-pre-redraw
  add-zle-hook-widget line-finish _keel-augment-line-finish
  _KEEL_AUGMENT_ACTIVE=1
  _keel-augment-render-current
  return 0
}

function keel-augment-disable() {
  (( _KEEL_AUGMENT_ACTIVE )) || return 0

  autoload -Uz add-zle-hook-widget
  add-zle-hook-widget -d line-pre-redraw _keel-augment-pre-redraw 2>/dev/null || true
  add-zle-hook-widget -d line-init _keel-augment-pre-redraw 2>/dev/null || true
  add-zle-hook-widget -d line-finish _keel-augment-line-finish 2>/dev/null || true

  local widget
  for widget in "${_KEEL_AUGMENT_SAVED_WIDGETS[@]}"; do
    zle -A ".keel-augment-original-$widget" "$widget" 2>/dev/null || true
    zle -D ".keel-augment-original-$widget" 2>/dev/null || true
  done
  _KEEL_AUGMENT_SAVED_WIDGETS=()

  RPROMPT="$_KEEL_AUGMENT_BASE_RPROMPT"
  ZLE_RPROMPT_INDENT="$_KEEL_AUGMENT_BASE_RPROMPT_INDENT"
  _KEEL_AUGMENT_ACTIVE=0
  return 0
}

function keel-augment-status() {
  if (( _KEEL_AUGMENT_ACTIVE )); then
    print "keel-augment: enabled"
    print "renderer: ratatui offscreen buffer -> zsh RPROMPT"
    print "host: zsh ZLE owns editing, cursor, resize, and command execution"
  else
    print "keel-augment: disabled"
  fi
}

if [[ -o interactive ]]; then
  keel-augment-enable
fi
