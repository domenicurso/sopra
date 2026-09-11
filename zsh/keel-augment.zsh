#!/usr/bin/env zsh

if [[ -n "${_KEEL_AUGMENT_LOADED:-}" ]]; then
  return 0
fi

typeset -g _KEEL_AUGMENT_LOADED=1
typeset -g _KEEL_AUGMENT_ROOT="${KEEL_AUGMENT_ROOT:-${${(%):-%N}:A:h:h}}"
typeset -g _KEEL_AUGMENT_BINARY="${KEEL_AUGMENT_BINARY:-$_KEEL_AUGMENT_ROOT/target/debug/keel-augment}"
typeset -g _KEEL_AUGMENT_ACTIVE=0
typeset -g _KEEL_AUGMENT_SERVER_RUNNING=0
typeset -g _KEEL_AUGMENT_SERVER_PID=''
typeset -g _KEEL_AUGMENT_SERVER_DIR=''
typeset -g _KEEL_AUGMENT_SERVER_IN_FD=''
typeset -g _KEEL_AUGMENT_SERVER_OUT_FD=''
typeset -g _KEEL_AUGMENT_BASE_PROMPT=''
typeset -g _KEEL_AUGMENT_BASE_RPROMPT=''
typeset -g _KEEL_AUGMENT_BASE_RPROMPT_INDENT=1
typeset -g _KEEL_AUGMENT_LAST_FRAGMENT=''
typeset -ga _KEEL_AUGMENT_LAST_HIGHLIGHTS=()
typeset -g _KEEL_AUGMENT_CURSOR_MODE="${KEEL_AUGMENT_CURSOR_MODE:-native}"
typeset -g _KEEL_AUGMENT_CURSOR_STYLE='default'
typeset -g _KEEL_AUGMENT_POC="${KEEL_AUGMENT_POC:-status}"
typeset -g _KEEL_AUGMENT_PROMPT_MODE="${KEEL_AUGMENT_PROMPT_MODE:-base}"
if [[ -z "${KEEL_AUGMENT_PROMPT_MODE:-}" ]]; then
  case "$_KEEL_AUGMENT_POC" in
    prompt|dashboard|all) _KEEL_AUGMENT_PROMPT_MODE=rust ;;
  esac
fi
typeset -g _KEEL_AUGMENT_IN_REDRAW=0
typeset -g _KEEL_AUGMENT_LAST_STATUS=0
typeset -g _KEEL_AUGMENT_LAST_COMMAND=''

function _keel-augment-escape() {
  REPLY="$1"
  REPLY=${REPLY//\\/\\\\}
  REPLY=${REPLY//$'\t'/\\t}
  REPLY=${REPLY//$'\n'/\\n}
  REPLY=${REPLY//$'\r'/\\r}
}

function _keel-augment-start-server() {
  (( _KEEL_AUGMENT_SERVER_RUNNING )) && return 0

  local directory="${TMPDIR:-/tmp}/keel-augment-${$}-${RANDOM}"
  local request_fifo="$directory/request"
  local response_fifo="$directory/response"
  mkdir -m 700 "$directory" 2>/dev/null || return 1
  if ! mkfifo "$request_fifo" "$response_fifo" 2>/dev/null; then
    rmdir "$directory" 2>/dev/null || true
    return 1
  fi

  local input_fd output_fd
  if ! exec {input_fd}<>"$request_fifo"; then
    rm -f "$request_fifo" "$response_fifo"
    rmdir "$directory" 2>/dev/null || true
    return 1
  fi
  if ! exec {output_fd}<>"$response_fifo"; then
    exec {input_fd}>&-
    rm -f "$request_fifo" "$response_fifo"
    rmdir "$directory" 2>/dev/null || true
    return 1
  fi

  "$_KEEL_AUGMENT_BINARY" --server <&$input_fd >&$output_fd 2>/dev/null &!
  _KEEL_AUGMENT_SERVER_PID=$!
  _KEEL_AUGMENT_SERVER_DIR="$directory"
  _KEEL_AUGMENT_SERVER_IN_FD=$input_fd
  _KEEL_AUGMENT_SERVER_OUT_FD=$output_fd
  _KEEL_AUGMENT_SERVER_RUNNING=1
}

function _keel-augment-stop-server() {
  local pid="$_KEEL_AUGMENT_SERVER_PID"
  local directory="$_KEEL_AUGMENT_SERVER_DIR"
  local request_fifo="$directory/request"
  local response_fifo="$directory/response"
  local input_fd="$_KEEL_AUGMENT_SERVER_IN_FD"
  local output_fd="$_KEEL_AUGMENT_SERVER_OUT_FD"
  if (( _KEEL_AUGMENT_SERVER_RUNNING )); then
    print -r -u "$input_fd" -- shutdown 2>/dev/null || true
  fi
  local response
  if (( _KEEL_AUGMENT_SERVER_RUNNING )); then
    read -r -t 1 -u "$output_fd" response 2>/dev/null || true
  fi
  [[ -n "$input_fd" ]] && exec {input_fd}>&-
  [[ -n "$output_fd" ]] && exec {output_fd}>&-
  [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
  rm -f "$request_fifo" "$response_fifo" 2>/dev/null || true
  [[ -n "$directory" ]] && rmdir "$directory" 2>/dev/null || true
  _KEEL_AUGMENT_SERVER_PID=''
  _KEEL_AUGMENT_SERVER_DIR=''
  _KEEL_AUGMENT_SERVER_IN_FD=''
  _KEEL_AUGMENT_SERVER_OUT_FD=''
  _KEEL_AUGMENT_SERVER_RUNNING=0
}

function _keel-augment-server-failed() {
  local pid="$_KEEL_AUGMENT_SERVER_PID"
  local directory="$_KEEL_AUGMENT_SERVER_DIR"
  local request_fifo="$directory/request"
  local response_fifo="$directory/response"
  local input_fd="$_KEEL_AUGMENT_SERVER_IN_FD"
  local output_fd="$_KEEL_AUGMENT_SERVER_OUT_FD"
  [[ -n "$input_fd" ]] && exec {input_fd}>&-
  [[ -n "$output_fd" ]] && exec {output_fd}>&-
  _KEEL_AUGMENT_SERVER_PID=''
  _KEEL_AUGMENT_SERVER_DIR=''
  _KEEL_AUGMENT_SERVER_IN_FD=''
  _KEEL_AUGMENT_SERVER_OUT_FD=''
  _KEEL_AUGMENT_SERVER_RUNNING=0
  if [[ -n "$pid" ]]; then
    kill "$pid" 2>/dev/null || true
  fi
  rm -f "$request_fifo" "$response_fifo" 2>/dev/null || true
  [[ -n "$directory" ]] && rmdir "$directory" 2>/dev/null || true
}

function _keel-augment-request() {
  (( _KEEL_AUGMENT_SERVER_RUNNING )) || return 1

  local buffer="$1"
  local cursor="$2"
  local columns="$3"
  local rows="$4"
  local keymap="$5"
  local exit_status="$6"
  local cwd="${PWD:-~}"
  local tab=$'\t'
  local encoded_buffer
  local encoded_keymap
  local response

  _keel-augment-escape "$buffer"
  encoded_buffer="$REPLY"
  _keel-augment-escape "$keymap"
  encoded_keymap="$REPLY"
  _keel-augment-escape "$cwd"
  local encoded_cwd="$REPLY"

  print -r -u "$_KEEL_AUGMENT_SERVER_IN_FD" -- "render${tab}${encoded_buffer}${tab}${cursor}${tab}${columns}${tab}${rows}${tab}${encoded_keymap}${tab}${exit_status}${tab}${encoded_cwd}" 2>/dev/null || return 1
  read -r -t 1 -u "$_KEEL_AUGMENT_SERVER_OUT_FD" response 2>/dev/null || return 1
  REPLY="$response"
}

function _keel-augment-unescape() {
  REPLY="$1"
  REPLY=${REPLY//\\t/$'\t'}
  REPLY=${REPLY//\\n/$'\n'}
  REPLY=${REPLY//\\r/$'\r'}
  REPLY=${REPLY//\\\\/\\}
}

function _keel-augment-remove-highlight() {
  (( ${#_KEEL_AUGMENT_LAST_HIGHLIGHTS} )) || return 0
  local -a next=()
  local item owned previous_entry
  for item in "${region_highlight[@]}"; do
    owned=0
    for previous_entry in "${_KEEL_AUGMENT_LAST_HIGHLIGHTS[@]}"; do
      if [[ "$item" == "$previous_entry" ]]; then
        owned=1
        break
      fi
    done
    (( owned )) || next+=("$item")
  done
  region_highlight=("${next[@]}")
  _KEEL_AUGMENT_LAST_HIGHLIGHTS=()
}

function _keel-augment-apply-highlights() {
  local payload="$1"
  _keel-augment-remove-highlight
  [[ -n "$payload" ]] || return 0

  local -a encoded=()
  encoded=("${(@s:;:)payload}")
  local item start end style rest
  for item in "${encoded[@]}"; do
    start="${item%%,*}"
    rest="${item#*,}"
    end="${rest%%,*}"
    style="${rest#*,}"
    [[ "$start" == <-> && "$end" == <-> && "$start" -lt "$end" ]] || continue
    _KEEL_AUGMENT_LAST_HIGHLIGHTS+=("$start $end $style")
    region_highlight+=("$start $end $style")
  done
}

function _keel-augment-apply-cursor-style() {
  local style="$1"
  [[ "$_KEEL_AUGMENT_CURSOR_MODE" == off ]] && style=default
  [[ "$style" == "$_KEEL_AUGMENT_CURSOR_STYLE" ]] && return 0

  local sequence
  case "$style" in
    default) sequence=$'\e[0 q' ;;
    blink-block) sequence=$'\e[1 q' ;;
    block) sequence=$'\e[2 q' ;;
    blink-underline) sequence=$'\e[3 q' ;;
    underline) sequence=$'\e[4 q' ;;
    blink-bar) sequence=$'\e[5 q' ;;
    bar) sequence=$'\e[6 q' ;;
    *) style=default; sequence=$'\e[0 q' ;;
  esac

  if [[ -t 1 ]]; then
    print -n -u 1 -- "$sequence"
  fi
  _KEEL_AUGMENT_CURSOR_STYLE="$style"
}

function _keel-augment-render() {
  local response
  _keel-augment-request "${1:-}" "${2:-0}" "${3:-80}" "${4:-24}" "${5:-main}" "${6:-0}" || {
    PROMPT="$_KEEL_AUGMENT_BASE_PROMPT"
    RPROMPT="$_KEEL_AUGMENT_BASE_RPROMPT"
    _KEEL_AUGMENT_LAST_FRAGMENT=''
    _keel-augment-remove-highlight
    _keel-augment-apply-cursor-style default
    _keel-augment-server-failed
    _keel-augment-start-server >/dev/null 2>&1 || true
    return 0
  }
  response="$REPLY"
  local -a fields
  fields=("${(@ps:\t:)response}")
  (( ${#fields} >= 4 )) || return 1

  local prompt_fragment right_fragment
  _keel-augment-unescape "${fields[1]}"
  prompt_fragment="$REPLY"
  _keel-augment-unescape "${fields[2]}"
  right_fragment="$REPLY"
  local cursor_style="${fields[3]}"
  local highlight_payload="${fields[4]}"

  if [[ "$_KEEL_AUGMENT_PROMPT_MODE" == rust ]]; then
    PROMPT="${prompt_fragment} "
  else
    PROMPT="$_KEEL_AUGMENT_BASE_PROMPT"
  fi
  RPROMPT="${_KEEL_AUGMENT_BASE_RPROMPT}${right_fragment}"
  _KEEL_AUGMENT_LAST_FRAGMENT="$right_fragment"
  _keel-augment-apply-highlights "$highlight_payload"
  _keel-augment-apply-cursor-style "$cursor_style"
}

function _keel-augment-pre-redraw() {
  (( _KEEL_AUGMENT_ACTIVE )) || return 0
  (( _KEEL_AUGMENT_IN_REDRAW )) && return 0
  local previous_prompt="$PROMPT"
  local previous_rprompt="$RPROMPT"
  _KEEL_AUGMENT_IN_REDRAW=1
  _keel-augment-render "${BUFFER:-}" "${CURSOR:-0}" "${COLUMNS:-80}" "${LINES:-24}" "${KEYMAP:-main}" "${_KEEL_AUGMENT_LAST_STATUS:-0}"
  if [[ "$PROMPT" != "$previous_prompt" || "$RPROMPT" != "$previous_rprompt" ]]; then
    zle reset-prompt 2>/dev/null || true
  fi
  _KEEL_AUGMENT_IN_REDRAW=0
  return 0
}

function _keel-augment-preexec() {
  _KEEL_AUGMENT_LAST_COMMAND="${1:-}"
}

function _keel-augment-precmd() {
  _KEEL_AUGMENT_LAST_STATUS=$?
  (( _KEEL_AUGMENT_ACTIVE )) || return 0
  _keel-augment-render '' 0 "${COLUMNS:-80}" "${LINES:-24}" "${KEYMAP:-main}" "${_KEEL_AUGMENT_LAST_STATUS:-0}"
}

function _keel-augment-line-finish() {
  (( _KEEL_AUGMENT_ACTIVE )) || return 0
  RPROMPT="$_KEEL_AUGMENT_BASE_RPROMPT"
  _KEEL_AUGMENT_LAST_FRAGMENT=''
  _keel-augment-remove-highlight
  _keel-augment-apply-cursor-style default
  return 0
}

function _keel-augment-zshexit() {
  (( _KEEL_AUGMENT_ACTIVE )) || return 0
  keel-augment-disable
}

function keel-augment-enable() {
  (( _KEEL_AUGMENT_ACTIVE )) && return 0

  if [[ ! -x "$_KEEL_AUGMENT_BINARY" ]]; then
    print -u2 "keel-augment: build $_KEEL_AUGMENT_BINARY first"
    return 1
  fi

  zmodload -i zsh/zle
  autoload -Uz add-zle-hook-widget add-zsh-hook
  _KEEL_AUGMENT_BASE_PROMPT="${PROMPT-}"
  _KEEL_AUGMENT_BASE_RPROMPT="${RPROMPT-}"
  _KEEL_AUGMENT_BASE_RPROMPT_INDENT="${ZLE_RPROMPT_INDENT:-1}"
  ZLE_RPROMPT_INDENT=0
  _keel-augment-start-server || {
    ZLE_RPROMPT_INDENT="$_KEEL_AUGMENT_BASE_RPROMPT_INDENT"
    return 1
  }

  add-zle-hook-widget line-pre-redraw _keel-augment-pre-redraw
  add-zle-hook-widget line-finish _keel-augment-line-finish
  add-zsh-hook precmd _keel-augment-precmd
  add-zsh-hook preexec _keel-augment-preexec
  add-zsh-hook zshexit _keel-augment-zshexit
  _KEEL_AUGMENT_ACTIVE=1
  _keel-augment-render '' 0 "${COLUMNS:-80}" "${LINES:-24}" "${KEYMAP:-main}" "${_KEEL_AUGMENT_LAST_STATUS:-0}"
  return 0
}

function keel-augment-disable() {
  (( _KEEL_AUGMENT_ACTIVE )) || return 0

  autoload -Uz add-zle-hook-widget add-zsh-hook
  add-zle-hook-widget -d line-pre-redraw _keel-augment-pre-redraw 2>/dev/null || true
  add-zle-hook-widget -d line-finish _keel-augment-line-finish 2>/dev/null || true
  add-zsh-hook -d precmd _keel-augment-precmd 2>/dev/null || true
  add-zsh-hook -d preexec _keel-augment-preexec 2>/dev/null || true
  add-zsh-hook -d zshexit _keel-augment-zshexit 2>/dev/null || true

  _keel-augment-stop-server
  PROMPT="$_KEEL_AUGMENT_BASE_PROMPT"
  RPROMPT="$_KEEL_AUGMENT_BASE_RPROMPT"
  ZLE_RPROMPT_INDENT="$_KEEL_AUGMENT_BASE_RPROMPT_INDENT"
  _KEEL_AUGMENT_LAST_FRAGMENT=''
  _keel-augment-remove-highlight
  _keel-augment-apply-cursor-style default
  _KEEL_AUGMENT_ACTIVE=0
  return 0
}

function keel-augment-status() {
  if (( _KEEL_AUGMENT_ACTIVE )); then
    print "keel-augment: enabled"
    print "renderer: persistent Rust process -> ratatui buffer -> zsh prompt surfaces"
    print "host: zsh ZLE owns editing, resize, and command execution"
    print "profile: $_KEEL_AUGMENT_POC prompt=$_KEEL_AUGMENT_PROMPT_MODE"
    print "cursor: mode=$_KEEL_AUGMENT_CURSOR_MODE style=$_KEEL_AUGMENT_CURSOR_STYLE"
    if (( _KEEL_AUGMENT_SERVER_RUNNING )); then
      print -r -u "$_KEEL_AUGMENT_SERVER_IN_FD" -- stats 2>/dev/null || true
      local response
      if read -r -t 1 -u "$_KEEL_AUGMENT_SERVER_OUT_FD" response 2>/dev/null; then
        print "server: $response"
      fi
    else
      print "server: unavailable; using the base prompt"
    fi
  else
    print "keel-augment: disabled"
  fi
}

if [[ -o interactive ]]; then
  keel-augment-enable
fi
