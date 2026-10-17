#!/usr/bin/env bash
# Registered operator entry points always use the existing lock/token wrapper.
set -euo pipefail

deployctl_exec() { exec /usr/local/sbin/silkboard-update --json --profile silkboard "$@"; }

deployctl_main() {
    [[ $# -ge 2 ]] || { echo 'usage: deployctl poll|status|inspect|retry PROFILE [TAG]' >&2; return 2; }
    local operation=$1 profile=$2
    shift 2
    [[ $profile == silkboard ]] || { echo 'application profile is not registered' >&2; return 2; }
    case "$operation" in
        poll|status) [[ $# == 0 ]] || return 2 ;;
        inspect|retry) [[ $# == 1 ]] || return 2 ;;
        *) echo 'unknown deployment operation' >&2; return 2 ;;
    esac
    deployctl_exec "$operation" "$@"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then deployctl_main "$@"; fi
