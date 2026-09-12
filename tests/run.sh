#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
"$repo_root/scripts/build-keel.sh"
KEEL_MODULE_PATH="$repo_root/target/debug" \
    "$repo_root/target/keel-zsh/bin/zsh" -dfc \
    'module_path=($KEEL_MODULE_PATH $module_path); zmodload zsh/zle; zmodload keel'
expect "$repo_root/tests/native.exp" "$repo_root" "$repo_root/target/keel-zsh/bin/zsh"
