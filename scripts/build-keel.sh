#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
progress() {
    if [[ ${KEEL_PROGRESS:-0} == 1 ]]; then
        printf 'keel-progress:%s\n' "$*"
    fi
}

progress 'Preparing patched Zsh'
"$repo_root/scripts/build-zsh.sh"
progress 'Compiling Rust module'
cargo build --manifest-path "$repo_root/Cargo.toml" -p keel-module

rust_archive="$repo_root/target/debug/libkeel_module.a"
output="$repo_root/target/debug/keel.so"
cc=${CC:-cc}
native_objects=()
native_sources=(
    keel_zsh_module.c
    keel_zsh_render.c
    keel_zsh_widgets.c
    keel_zsh_completion_capture.c
    keel_zsh_completion_worker.c
)

source_count=${#native_sources[@]}
source_index=0
for source in "${native_sources[@]}"; do
    source_index=$((source_index + 1))
    object="$repo_root/target/debug/${source%.c}.o"
    progress "Compiling native module ($source_index/$source_count): $source"
    "$cc" -std=c11 -Wall -Wextra -Werror -fPIC \
        -I "$repo_root/native" \
        -c "$repo_root/native/$source" \
        -o "$object"
    native_objects+=("$object")
done

progress 'Linking Keel module'
case "$(uname -s)" in
    Darwin)
        "$cc" -bundle -flat_namespace -undefined suppress \
            -o "$output" \
            "${native_objects[@]}" \
            "$rust_archive" -lpthread
        ;;
    Linux)
        "$cc" -shared -Wl,--unresolved-symbols=ignore-in-shared-libs \
            -o "$output" \
            "${native_objects[@]}" \
            "$rust_archive" -lpthread -ldl -lm
        ;;
    *)
        echo "keel: unsupported host OS: $(uname -s)" >&2
        exit 1
        ;;
esac

progress "Built Keel module: $output"
printf 'built %s\n' "$output"
