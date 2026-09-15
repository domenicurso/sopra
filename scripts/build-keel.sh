#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cargo build --manifest-path "$repo_root/Cargo.toml" --bin keel-demo
printf 'built %s\n' "$repo_root/target/debug/keel-demo"
