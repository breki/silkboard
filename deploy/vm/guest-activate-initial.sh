#!/usr/bin/env bash
# Operator-only first activation. Upgrades/recovery belong to issue #6.
set -euo pipefail
umask 022
fail() { echo "silkboard activation: $*" >&2; exit 1; }
cleanup() {
    local status=$?
    trap - EXIT
    if [[ ${activation_pending:-0} == 1 ]]; then
        systemctl disable --now silkboard || true
        rm -f -- "$current"
    fi
    [[ -z ${archive:-} ]] || rm -f -- "$archive"
    exit "$status"
}
start_initial() {
    activation_pending=1
    trap cleanup EXIT
    systemctl enable --now silkboard
    for ((attempt=0; attempt<30; attempt++)); do
        if curl -fsS --max-time 2 http://127.0.0.1:3000/api/status |
            jq -e --arg tag "$tag" --arg commit "$commit" \
                '.health=="ok" and .release_tag==$tag and .commit==$commit and ("v"+.version)==$tag' >/dev/null &&
            curl -fsS --max-time 2 http://127.0.0.1:3000/health; then
            activation_pending=0
            echo "Initial release $tag ($commit) healthy."
            return 0
        fi
        sleep 1
    done
    fail 'initial health check failed; staged release retained'
}
main() {
    [[ $# == 2 ]] || fail 'usage: guest-activate-initial.sh TAG COMMIT'
    [[ $EUID == 0 && $(hostname -s) == silkboard ]] || fail 'run as root in silkboard'
    tag=$1 commit=$2
    [[ $tag =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ && $commit =~ ^[0-9a-f]{40}$ ]] ||
        fail 'stable tag and full commit required'
    current=/opt/silkboard/current
    [[ ! -e $current && ! -L $current ]] ||
        fail 'a release already exists; this command never upgrades or replaces it'
    # Shared staging is installed alongside this operator-owned script.
    # shellcheck source=/dev/null
    source "$(dirname -- "${BASH_SOURCE[0]}")/guest-stage.sh"
    stage_release "$tag" "$commit"
    release=/opt/silkboard/releases/$tag-$commit
    ln -s -- "$release" "$current"
    start_initial
}
# Sourcing defines testable lifecycle functions without invoking root operations.
if [[ ${BASH_SOURCE[0]} == "$0" ]]; then main "$@"; fi
