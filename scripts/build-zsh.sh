#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
version=5.9
prefix=${KEEL_ZSH_PREFIX:-"$repo_root/target/keel-zsh"}
archive="$repo_root/target/zsh-$version.tar.xz"
marker="$prefix/.keel-dynamic-$version-v18"

progress() {
    if [[ ${KEEL_PROGRESS:-0} == 1 ]]; then
        printf 'keel-progress:%s\n' "$*"
    fi
}

if [[ -x "$prefix/bin/zsh" && -f "$marker" ]]; then
    progress 'Using cached patched Zsh 5.9'
    exit 0
fi

mkdir -p "$repo_root/target"
if [[ ! -f "$archive" ]]; then
    progress 'Downloading Zsh 5.9 source'
    download_archive=$(mktemp "$repo_root/target/zsh-$version-download.XXXXXX")
    curl --fail --location --silent --show-error \
        "https://sourceforge.net/projects/zsh/files/zsh/$version/zsh-$version.tar.xz/download" \
        --output "$download_archive"
    mv "$download_archive" "$archive"
else
    progress 'Using cached Zsh 5.9 source archive'
fi

build_root=$(mktemp -d "$repo_root/target/zsh-build.XXXXXX")
trap 'rm -rf "$build_root"' EXIT
progress 'Extracting Zsh 5.9 source'
tar --extract --xz --file "$archive" --directory "$build_root"
source_root="$build_root/zsh-$version"

progress 'Patching Zsh redraw hooks'
perl "$repo_root/scripts/patch-zsh.pl" "$source_root"

# Zsh 5.9's configure probes use pre-C99 function declarations. Newer Apple
# Clang rejects those probes before they can report the platform capability.
if [[ -n ${CFLAGS:-} ]]; then
    zsh_cflags=$CFLAGS
else
    zsh_cflags='-Wall -Wmissing-prototypes -O2'
fi
zsh_cflags+=' -Wno-implicit-int -Wno-deprecated-non-prototype'
zsh_cflags+=' -Wno-error=implicit-int -Wno-error=deprecated-non-prototype'

case "$(uname -s)" in
    Darwin)
        zsh_dlldflags='-dynamiclib -install_name @rpath/libzsh-5.9.so -undefined dynamic_lookup'
        ;;
    Linux)
        zsh_dlldflags='-shared -Wl,-soname,libzsh-5.9.so'
        ;;
    *)
        printf 'keel: unsupported host OS: %s\n' "$(uname -s)" >&2
        exit 1
        ;;
esac

progress 'Configuring Zsh'
(cd "$source_root" && \
    zsh_cv_shared_environ=yes \
    zsh_cv_shared_tgetent=yes \
    zsh_cv_shared_tigetstr=yes \
    CFLAGS="$zsh_cflags" \
    DLLDFLAGS="$zsh_dlldflags" \
    ./configure --prefix="$prefix" --enable-multibyte --enable-dynamic \
        --with-tcsetpgrp)

if [[ -n "${JOBS:-}" ]]; then
    jobs=$JOBS
elif [[ "$(uname -s)" == Darwin ]] && command -v sysctl >/dev/null 2>&1; then
    jobs=$(sysctl -n hw.ncpu)
elif command -v getconf >/dev/null 2>&1 && jobs=$(getconf _NPROCESSORS_ONLN 2>/dev/null); then
    :
else
    jobs=2
fi

progress "Compiling Zsh with $jobs jobs"
(cd "$source_root" && make -j"$jobs")
progress 'Installing patched Zsh'
(cd "$source_root" && make install)
if [[ "$(uname -s)" == Darwin ]]; then
    progress 'Configuring Zsh loader path'
    install_name_tool -add_rpath '@loader_path/../lib/zsh' "$prefix/bin/zsh"
fi
progress 'Writing patched Zsh build marker'
printf '%s\n' "zsh=$version" "dynamic=1" "patch=semantic-anchors-v2" "abi=3" "module-exports=v1" "cflags=probe-compat-v1" "tcsetpgrp=assumed-v1" > "$marker"
