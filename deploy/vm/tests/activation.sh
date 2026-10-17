#!/usr/bin/env bash
# Run the actual lifecycle against temporary paths and fake service/HTTP edges.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/../../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
for scenario in startup-fails health-fails stop-fails success; do
    mkdir "$work/$scenario"
    ln -s retained-release "$work/$scenario/current"
    set +e
    bash -euo pipefail -c '
        source "$1/deploy/vm/guest-activate-initial.sh"
        current=$2/current
        archive=$2/archive
        touch "$archive"
        tag=v0.1.1 commit=c68a53902d8e9552abee2d3488645800f856986e
        scenario=$3
        # Functions inherit fixture variables, with no real systemctl or curl.
        systemctl() {
            printf "%s\n" "$*" >> "${current%/*}/service.log"
            if [[ $1 == enable && $scenario == startup-fails ]] ||
                [[ $1 == disable && $scenario == stop-fails ]]; then return 9; fi
        }
        curl() {
            if [[ ${*: -1} == */api/status ]]; then
                printf '\''{"health":"ok","release_tag":"v0.1.1","version":"0.1.1","commit":"%s"}\n'\'' "$commit"
            elif [[ $scenario == health-fails || $scenario == stop-fails ]]; then
                return 22
            else echo ok; fi
        }
        sleep() { :; }
        start_initial
    ' bash "$root" "$work/$scenario" "$scenario" > "$work/$scenario/output" 2>&1
    status=$?
    set -e
    [[ ! -e $work/$scenario/archive ]]
    if [[ $scenario == success ]]; then
        [[ $status == 0 && -L $work/$scenario/current ]]
        if grep -q '^disable' "$work/$scenario/service.log"; then exit 1; fi
    else
        [[ $status != 0 && ! -L $work/$scenario/current ]]
        grep -q '^disable --now silkboard$' "$work/$scenario/service.log"
    fi
done
echo 'Initial activation: startup/health/cleanup failures and success passed.'
