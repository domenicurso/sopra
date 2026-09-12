#!/usr/bin/env bash
set -euo pipefail

install_prefix=${KEEL_INSTALL_PREFIX:-"$HOME/.local/keel"}
marker="$install_prefix/.keel-install"

if [[ ! -e "$install_prefix" ]]; then
    printf 'Keel is not installed at %s\n' "$install_prefix"
    exit 0
fi

if [[ ! -f "$marker" || $(<"$marker") != 'keel-install-v1' ]]; then
    printf 'keel: refusing to remove %s; Keel install marker not found\n' "$install_prefix" >&2
    exit 1
fi

rm -rf -- "$install_prefix"
printf 'Keel removed from %s\n' "$install_prefix"
