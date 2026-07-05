#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
zsh_version="${KEEL_ZSH_VERSION:-5.9}"
zsh_src_root="$repo_root/target/zsh-src"
zsh_src_dir="$zsh_src_root/zsh-$zsh_version"
module_out_dir="$repo_root/target/zsh"
module_out="$module_out_dir/keel.so"
src_build_dir="$zsh_src_dir/Src"

mkdir -p "$zsh_src_root" "$module_out_dir"

if [[ ! -d "$zsh_src_dir" ]]; then
  archive="$zsh_src_root/zsh-$zsh_version.tar.xz"
  curl -L "https://downloads.sourceforge.net/project/zsh/zsh/$zsh_version/zsh-$zsh_version.tar.xz" -o "$archive"
  tar -xf "$archive" -C "$zsh_src_root"
fi

if [[ ! -f "$zsh_src_dir/config.h" ]]; then
  (
    cd "$zsh_src_dir"
    ./configure --prefix="$zsh_src_dir/.local"
  )
fi

make -C "$zsh_src_dir/Src" -f Makemod zsh.mdh proto.zsh
make -C "$zsh_src_dir/Src/Zle" -f Makefile zle.mdh proto.zle

cargo build -p keel-zsh-bridge -p keel-cli --bin keel-shell

cc_bin="$(awk -F'=' '/^CC[[:space:]]*=/{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
zsh_curses_h="$(awk -F'=' '/^ZSH_CURSES_H[[:space:]]*=/{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
zsh_term_h="$(awk -F'=' '/^ZSH_TERM_H[[:space:]]*=/{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
cppflags="$(awk -F'=' '/^CPPFLAGS[[:space:]]*=/{sub(/^[[:space:]]*/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
defs="$(awk -F'=' '/^DEFS[[:space:]]*=/{sub(/^[[:space:]]*/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
cflags="$(awk -F'=' '/^CFLAGS[[:space:]]*=/{sub(/^[[:space:]]*/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
dlcflags="$(awk -F'=' '/^DLCFLAGS[[:space:]]*=/{sub(/^[[:space:]]*/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
ldflags="$(awk -F'=' '/^LDFLAGS[[:space:]]*=/{sub(/^[[:space:]]*/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
dlldflags="$(awk -F'=' '/^DLLDFLAGS[[:space:]]*=/{sub(/^[[:space:]]*/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"
libs="$(awk -F'=' '/^LIBS[[:space:]]*=/{sub(/^[[:space:]]*/, "", $2); print $2; exit}' "$zsh_src_dir/Config/defs.mk")"

if [[ -z "$cc_bin" ]]; then
  echo "failed to resolve zsh module compiler" >&2
  exit 1
fi

if [[ ! -f "$src_build_dir/zshcurses.h" ]]; then
  if [[ -n "$zsh_curses_h" ]]; then
    printf '#include <%s>\n' "$zsh_curses_h" >"$src_build_dir/zshcurses.h"
  else
    : >"$src_build_dir/zshcurses.h"
  fi
fi

if [[ ! -f "$src_build_dir/zshterm.h" ]]; then
  if [[ -n "$zsh_term_h" ]]; then
    printf '#include <%s>\n' "$zsh_term_h" >"$src_build_dir/zshterm.h"
  else
    : >"$src_build_dir/zshterm.h"
  fi
fi

(
  cd "$repo_root"
  eval "\"$cc_bin\"" \
    $cppflags \
    $defs \
    -DMODULE \
    $cflags \
    $dlcflags \
    -I"$zsh_src_dir" \
    -I"$zsh_src_dir/Src" \
    -I"$zsh_src_dir/Src/Zle" \
    "$repo_root/zsh-module/keel_module.c" \
    "$repo_root/target/debug/libkeel_zsh_bridge.a" \
    $ldflags \
    $dlldflags \
    $libs \
    -o "$module_out"
)

printf '%s\n' "$module_out"
