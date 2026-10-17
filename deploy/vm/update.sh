#!/usr/bin/env bash
# Serialize timer, inspection, retry and manual polls against the same lock.
set -euo pipefail
[[ $EUID == 0 && $(hostname -s) == silkboard ]] || exit 1
exec 9>/var/lib/silkboard-updater/update.lock
flock --wait 5 9
install -d -m 0750 -o root -g silkfetch /run/silkboard-fetch
credential=/etc/silkboard/updater-token
rm -f /run/silkboard-fetch/token
if [[ -r $credential ]]; then
    install -o root -g silkfetch -m 0440 "$credential" /run/silkboard-fetch/token
else
    echo 'GitHub credential unavailable; recovery/status remain available.' >&2
fi
trap 'rm -f /run/silkboard-fetch/token' EXIT
/usr/local/libexec/silkboard/updater "$@"
