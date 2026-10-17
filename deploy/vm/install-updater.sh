#!/usr/bin/env bash
# Operator installation only; does not enable the timer or deploy an application.
set -euo pipefail
[[ $# == 2 && $EUID == 0 && $(hostname -s) == silkboard && $(uname -m) == aarch64 ]] ||
    { echo 'usage: install-updater.sh ARM64_BINARY TOKEN_FILE (root in silkboard)' >&2; exit 1; }
binary=$1 credential=$2
[[ -f $binary && -f $credential && ! -L $credential ]] || exit 1
[[ $(stat -c %a "$credential") == 600 && $(stat -c %u "$credential") == 0 ]] ||
    { echo 'require root-owned mode 600 token file' >&2; exit 1; }
grep -qE '^github_pat_[A-Za-z0-9_]+$' "$credential" || exit 1
directory=$(cd -- "$(dirname -- "$0")" && pwd)
if ! id silkfetch >/dev/null 2>&1; then
    useradd --system --home-dir /var/lib/silkboard-fetch --shell /usr/sbin/nologin silkfetch
fi
install -d -m 0700 /var/lib/silkboard-updater
exec 9>/var/lib/silkboard-updater/update.lock
flock --wait 5 9
install -d -m 0700 -o silkfetch -g silkfetch /var/lib/silkboard-fetch
install -d -m 0750 -o root -g silkbuild /var/lib/silkboard-input
install -d -m 0755 /usr/local/libexec/silkboard
install -m 0755 "$binary" /usr/local/libexec/silkboard/updater
for pair in 'guest-build.sh build' 'guest-stage.sh stage' 'github-read.sh github-read' 'git-credential.sh git-credential' 'snapshot-bundle.sh snapshot-bundle'; do
    read -r source destination <<< "$pair"
    install -m 0755 "$directory/$source" "/usr/local/libexec/silkboard/$destination"
done
install -m 0755 "$directory/update.sh" /usr/local/sbin/silkboard-update
install -m 0755 "$directory/deployctl.sh" /usr/local/sbin/deployctl
# Never overwrite an independently installed operator policy on upgrades.
if [[ ! -e /etc/silkboard/deployment.json && ! -L /etc/silkboard/deployment.json ]]; then
    install -m 0644 "$directory/deployment.json" /etc/silkboard/deployment.json
fi
install -m 0600 "$credential" /etc/silkboard/updater-token
if [[ ! -d /var/lib/silkboard-fetch/repo.git ]]; then
    runuser -u silkfetch -- git init --bare /var/lib/silkboard-fetch/repo.git
fi
install -m 0644 "$directory/silkboard-update.service" "$directory/silkboard-update.timer" /etc/systemd/system/
install -m 0644 "$directory/silkboard.service" /etc/systemd/system/silkboard.service
systemd-analyze verify /etc/systemd/system/silkboard-update.{service,timer}
systemctl daemon-reload
echo 'Updater installed, timer unchanged. Enable only after admission verification.'
