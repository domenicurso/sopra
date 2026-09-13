#!/usr/bin/env bash
set -euo pipefail

script_path=${BASH_SOURCE[0]:-}
if [[ -n $script_path && -f $script_path ]]; then
    repo_root=$(cd -- "$(dirname -- "$script_path")" && pwd)
    if [[ -f "$repo_root/scripts/install-keel.sh" ]]; then
        exec bash "$repo_root/scripts/install-keel.sh" "$@"
    fi
fi

keel_repository=${KEEL_REPOSITORY:-domenicurso/keel}
keel_ref=${KEEL_REF:-main}
source_url=${KEEL_SOURCE_URL:-"https://github.com/$keel_repository/archive/$keel_ref.tar.gz"}
source_root_dir=$(mktemp -d "${TMPDIR:-/tmp}/keel-source.XXXXXX")
source_archive="$source_root_dir/source.tar.gz"

cleanup() {
    rm -rf -- "$source_root_dir"
}
trap cleanup EXIT

printf 'Downloading Keel source from %s\n' "$source_url"
curl --fail --location --silent --show-error --retry 3 \
    "$source_url" \
    --output "$source_archive"
tar --extract --gzip --file "$source_archive" --directory "$source_root_dir"

repo_root=''
for candidate in "$source_root_dir"/*; do
    if [[ -x "$candidate/scripts/install-keel.sh" ]]; then
        repo_root=$candidate
        break
    fi
done
if [[ -z $repo_root && -x "$source_root_dir/scripts/install-keel.sh" ]]; then
    repo_root=$source_root_dir
fi
if [[ -z $repo_root || ! -x "$repo_root/scripts/install-keel.sh" ]]; then
    printf 'keel: downloaded source archive does not contain scripts/install-keel.sh\n' >&2
    exit 1
fi

"$repo_root/scripts/install-keel.sh" "$@" </dev/null
