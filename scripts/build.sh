#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cargo build --quiet --manifest-path "$repo_root/Cargo.toml" --bin sopra
