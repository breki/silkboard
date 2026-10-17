#!/usr/bin/env bash
# Operator bootstrap transport. Never install in the dashboard or build account.
set -euo pipefail
fail() { echo "silkboard agent: $*" >&2; exit 1; }
for tool in ssh jq base64 sha256sum; do command -v "$tool" >/dev/null || fail "missing $tool"; done
call() {
    local response
    # JSON travels on stdin, never as remote shell syntax. Domain/host are fixed.
    response=$(ssh -T -o BatchMode=yes -o ConnectTimeout=10 malina \
        'IFS= read -r request; sudo -n virsh --connect qemu:///system qemu-agent-command silkboard --timeout 10 "$request"' <<< "$1")
    jq -e 'if has("error") then error(.error.desc) else .return end' <<< "$response"
}
execute() {
    local request pid status deadline
    (($#)) || fail 'exec requires an executable and optional arguments'
    request=$(jq -cn --args '{execute:"guest-exec",arguments:{path:$ARGS.positional[0],arg:$ARGS.positional[1:],"capture-output":true}}' -- "$@")
    pid=$(call "$request" | jq -er '.pid')
    deadline=$(($(date +%s) + 1800))
    while (($(date +%s) < deadline)); do
        status=$(call "$(jq -cn --argjson pid "$pid" '{execute:"guest-exec-status",arguments:{pid:$pid}}')")
        if [[ $(jq -r '.exited' <<< "$status") == true ]]; then
            jq -r '."out-data" // ""' <<< "$status" | base64 --decode
            jq -r '."err-data" // ""' <<< "$status" | base64 --decode >&2
            if [[ $(jq -r '(."out-truncated" // false) or (."err-truncated" // false)' <<< "$status") == true ]]; then
                echo 'Guest output truncated; consult durable guest logs.' >&2
            fi
            return "$(jq -r '.exitcode // 1' <<< "$status")"
        fi
        sleep 1
    done
    fail "guest command PID $pid still running after 1800 seconds; inspect before retry"
}
case ${1:-} in
    exec) shift; execute "$@" ;;
    put)
        [[ $# == 3 && -f $2 && $3 =~ ^[a-zA-Z0-9._-]+$ && $3 != . && $3 != .. ]] ||
            fail 'put requires LOCAL_FILE and a simple destination filename'
        execute /usr/bin/install -d -m 0700 /tmp/silkboard-bootstrap
        target=/tmp/silkboard-bootstrap/$3
        handle=$(call "$(jq -cn --arg path "$target" '{execute:"guest-file-open",arguments:{path:$path,mode:"w"}}')")
        # Close the handle on a failed write as well as success.
        trap 'call "$(jq -cn --argjson handle "$handle" '\''{execute:"guest-file-close",arguments:{handle:$handle}}'\'')" >/dev/null || true' EXIT
        while IFS= read -r chunk || [[ -n $chunk ]]; do
            call "$(jq -cn --argjson handle "$handle" --arg data "$chunk" '{execute:"guest-file-write",arguments:{handle:$handle,"buf-b64":$data}}')" >/dev/null
        done < <(base64 -w 0 -- "$2" | fold -w 32768)
        call "$(jq -cn --argjson handle "$handle" '{execute:"guest-file-close",arguments:{handle:$handle}}')" >/dev/null
        trap - EXIT
        expected=$(sha256sum -- "$2")
        actual=$(execute /usr/bin/sha256sum "$target")
        [[ ${expected%% *} == "${actual%% *}" ]] || fail 'guest file checksum mismatch'
        echo "Transferred $target" ;;
    *) fail 'usage: guest-agent.sh exec PROGRAM [ARGS...] | put LOCAL_FILE FILENAME' ;;
esac
