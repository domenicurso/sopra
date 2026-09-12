#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
version=5.9
prefix=${KEEL_ZSH_PREFIX:-"$repo_root/target/keel-zsh"}
archive="$repo_root/target/zsh-$version.tar.xz"
marker="$prefix/.keel-dynamic-$version-v2"

if [[ -x "$prefix/bin/zsh" && -f "$marker" ]]; then
    exit 0
fi

mkdir -p "$repo_root/target"
if [[ ! -f "$archive" ]]; then
    curl --fail --location --silent --show-error \
        "https://sourceforge.net/projects/zsh/files/zsh/$version/zsh-$version.tar.xz/download" \
        --output "$archive"
fi

build_root=$(mktemp -d "$repo_root/target/zsh-build.XXXXXX")
trap 'rm -rf "$build_root"' EXIT
tar --extract --xz --file "$archive" --directory "$build_root"
source_root="$build_root/zsh-$version"

patch --directory "$source_root" --strip=1 \
    < "$repo_root/patches/zsh-5.9-keel-redraw.patch"
(cd "$source_root" && \
    zsh_cv_shared_environ=yes \
    zsh_cv_shared_tgetent=yes \
    zsh_cv_shared_tigetstr=yes \
    DLLDFLAGS='-dynamiclib -install_name @rpath/libzsh-5.9.so -undefined dynamic_lookup' \
    ./configure --prefix="$prefix" --enable-multibyte --enable-dynamic)

if [[ -n "${JOBS:-}" ]]; then
    jobs=$JOBS
elif command -v sysctl >/dev/null 2>&1; then
    jobs=$(sysctl -n hw.ncpu)
else
    jobs=2
fi

(cd "$source_root" && make -j"$jobs")
(cd "$source_root" && make install)
if [[ "$(uname -s)" == Darwin ]]; then
    install_name_tool -add_rpath '@loader_path/../lib/zsh' "$prefix/bin/zsh"
fi
printf '%s\n' "zsh=$version" "dynamic=1" "patch=zsh-5.9-keel-redraw" > "$marker"
