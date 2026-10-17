#!/usr/bin/env bash
# Exercise operator transport against a fake SSH boundary; never contact a host.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/../../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
mkdir "$work/bin"
cat > "$work/bin/ssh" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
read -r request
printf '%s\n' "$request" >> "$VM_TEST_LOG"
if [[ ${VM_TEST_ERROR:-} == 1 ]]; then
    echo '{"error":{"desc":"guest agent unavailable"}}'
else
    case $(jq -r .execute <<< "$request") in
        guest-exec) echo '{"return":{"pid":7}}' ;;
        guest-exec-status)
            jq -cn --argjson code "${VM_TEST_EXIT:-0}" \
                '{return:{exited:true,exitcode:$code,"out-data":"aGVsbG8K"}}' ;;
        *) exit 80 ;;
    esac
fi
MOCK
chmod +x "$work/bin/ssh"
export PATH="$work/bin:$PATH" VM_TEST_LOG="$work/requests"
transport=$root/deploy/vm/guest-agent.sh
# shellcheck disable=SC2016 # Literal shell syntax must survive as argument data.
payload='$(touch SHOULD_NOT_EXIST); quote'"'"' and --flags'
"$transport" exec /bin/bash -lc "$payload" > "$work/output"
[[ $(cat "$work/output") == hello ]]
jq -se --arg payload "$payload" \
    '.[0].arguments.path=="/bin/bash" and .[0].arguments.arg==["-lc",$payload]' \
    "$VM_TEST_LOG" >/dev/null
[[ ! -e SHOULD_NOT_EXIST ]]
set +e
VM_TEST_EXIT=23 "$transport" exec /bin/false > /dev/null
status=$?
set -e
[[ $status == 23 ]]
if VM_TEST_ERROR=1 "$transport" exec /bin/true > /dev/null 2>&1; then
    echo 'remote error was accepted' >&2; exit 1
fi
before=$(wc -l < "$VM_TEST_LOG")
if "$transport" put "$work/output" ../escape > /dev/null 2>&1; then
    echo 'destination traversal was accepted' >&2; exit 1
fi
[[ $(wc -l < "$VM_TEST_LOG") == "$before" ]]
echo 'Operator transport: argument boundaries, exit codes, errors and traversal passed.'
