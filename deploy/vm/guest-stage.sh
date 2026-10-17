#!/usr/bin/env bash
# Snapshot build artifacts without exposing root to builder-controlled links.
set -euo pipefail

snapshot_artifacts() (
    set -euo pipefail
    umask 022
    [[ $# == 4 ]] || exit 1
    source=$1 release=$2 tag=$3 commit=$4
    [[ $tag =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ && $commit =~ ^[0-9a-f]{40}$ ]] || exit 1
    [[ ! -e $release && ! -L $release ]] || exit 1
    archive=$(mktemp)
    trap 'rm -f -- "$archive"' EXIT
    # Fixed find roots, NUL names and dereferencing produce regular entries only.
    # shellcheck disable=SC2016 # Positional parameter belongs to the child shell.
    runuser -u silkbuild -- bash -c '
        set -euo pipefail
        cd -- "$1"
        find silkboard-web frontend identity -type f -print0 |
            tar --create --dereference --hard-dereference --null --verbatim-files-from \
                --no-recursion --files-from=- --file=-
    ' bash "$source" > "$archive"
    LC_ALL=C tar --list --verbose --quoting-style=c --file="$archive" |
        awk 'substr($0,1,1)!="-" {bad=1} END {exit bad}' || exit 1
    install -d -m 0700 "$release"
    tar --extract --file="$archive" --directory="$release" \
        --no-same-owner --no-same-permissions
    [[ -f $release/silkboard-web && -f $release/frontend/index.html ]] || exit 1
    [[ $(cat "$release/identity") == "$(printf '%s\n%s' "$tag" "$commit")" ]] || exit 1
    # Normalize before making staging accessible; discard set-ID bits.
    find "$release" -type f -exec chmod 0644 {} +
    find "$release" -type d -exec chmod 0755 {} +
    chmod 0755 "$release/silkboard-web"
)

stage_release() {
    [[ $# == 2 && $EUID == 0 && $(hostname -s) == silkboard ]] ||
        { echo 'stage: require root in silkboard and TAG COMMIT' >&2; return 1; }
    # Validate before constructing paths, not only inside the snapshot function.
    [[ $1 =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ && $2 =~ ^[0-9a-f]{40}$ ]] || return 1
    snapshot_artifacts "/var/lib/silkboard-build/output-$1" "/opt/silkboard/releases/$1-$2" "$1" "$2"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then stage_release "$@"; fi
