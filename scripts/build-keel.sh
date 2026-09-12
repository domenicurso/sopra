#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
"$repo_root/scripts/build-zsh.sh"
cargo build --manifest-path "$repo_root/Cargo.toml" -p keel-module

rust_archive="$repo_root/target/debug/libkeel_module.a"
output="$repo_root/target/debug/keel.so"
cc=${CC:-cc}
native_objects=()

for source in keel_zsh_module.c keel_zsh_render.c keel_zsh_widgets.c; do
    object="$repo_root/target/debug/${source%.c}.o"
    "$cc" -std=c11 -Wall -Wextra -Werror -fPIC \
        -I "$repo_root/native" \
        -c "$repo_root/native/$source" \
        -o "$object"
    native_objects+=("$object")
done

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

printf 'built %s\n' "$output"
