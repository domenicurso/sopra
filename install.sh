#!/usr/bin/env bash
# Install Sopra from the repository:
#   curl -fsSL https://raw.githubusercontent.com/domenicurso/sopra/main/install.sh | bash

set -euo pipefail

SOPRA_REPOSITORY=${SOPRA_REPOSITORY:-domenicurso/sopra}
SOPRA_REF=${SOPRA_REF:-main}
SOPRA_PREFIX=${SOPRA_PREFIX:-"${HOME}/.local"}

die() {
    printf 'sopra installer: %s\n' "$*" >&2
    exit 1
}

command -v curl >/dev/null 2>&1 || die "curl is required"
command -v cargo >/dev/null 2>&1 || die "Rust and Cargo are required; install them from https://rustup.rs"
command -v tar >/dev/null 2>&1 || die "tar is required"
command -v install >/dev/null 2>&1 || die "install is required"

[[ $SOPRA_REPOSITORY =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || die "invalid SOPRA_REPOSITORY: $SOPRA_REPOSITORY"
[[ $SOPRA_REF =~ ^[A-Za-z0-9][A-Za-z0-9._/+@-]*$ ]] || die "invalid SOPRA_REF: $SOPRA_REF"

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/sopra-install.XXXXXX")
cleanup() {
    rm -rf "$work_dir"
}
trap cleanup EXIT

source_dir="$work_dir/source"
mkdir -p "$source_dir"
archive_url="https://github.com/$SOPRA_REPOSITORY/archive/$SOPRA_REF.tar.gz"
printf 'Downloading Sopra from %s...\n' "$archive_url"
curl -fsSL "$archive_url" | tar -xzf - --strip-components=1 -C "$source_dir"

printf 'Building Sopra...\n'
(cd "$source_dir" && cargo build --release --locked --bin sopra)

mkdir -p "$SOPRA_PREFIX/bin" "$SOPRA_PREFIX/share/sopra"
install -m 0755 "$source_dir/target/release/sopra" "$SOPRA_PREFIX/bin/sopra"
install -m 0644 "$source_dir/zsh/editor.zsh" "$SOPRA_PREFIX/share/sopra/editor.zsh"

printf '\nSopra was installed to %s/bin/sopra\n' "$SOPRA_PREFIX"
printf 'Add %s/share/sopra/editor.zsh to your interactive Zsh configuration:\n' "$SOPRA_PREFIX"
printf '  source %s/share/sopra/editor.zsh\n' "$SOPRA_PREFIX"
printf 'Make sure %s/bin is on PATH before starting Zsh.\n' "$SOPRA_PREFIX"
