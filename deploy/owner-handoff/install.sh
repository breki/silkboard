#!/usr/bin/env bash
# Install only. Starting the listener is a separate explicit operator action.
set -euo pipefail
umask 077
export PATH=/usr/sbin:/usr/bin:/sbin:/bin
if [[ $EUID != 0 || $# != 2 || $1 != /* ]]; then
    echo 'usage: sudo install.sh /absolute/path/to/node /path/to/config.json' >&2
    exit 1
fi
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
node_binary=$1
config_source=$2
[[ -x $node_binary && -f $config_source ]]
if systemctl is-active --quiet silkboard-owner-handoff.service; then
    echo 'Stop the existing handoff before installing.' >&2
    exit 1
fi
work=$(mktemp -d)
trap 'rm -f -- "$work/config.json" "$work/handoff.service"; rmdir -- "$work"' EXIT
"$node_binary" "$source_dir/config.mjs" "$config_source" "$work"
install -d -m 0700 -o root -g root /usr/local/libexec/silkboard-owner-handoff
for module in config server runtime main; do
    install -m 0600 -o root -g root "$source_dir/$module.mjs" "/usr/local/libexec/silkboard-owner-handoff/$module.mjs"
done
install -m 0600 -o root -g root "$work/config.json" /etc/silkboard-owner-handoff.json
install -m 0600 -o root -g root "$work/handoff.service" /etc/systemd/system/silkboard-owner-handoff.service
systemd-analyze verify /etc/systemd/system/silkboard-owner-handoff.service
systemctl daemon-reload
printf 'Handoff helper installed; listener was not started or enabled.\n'
