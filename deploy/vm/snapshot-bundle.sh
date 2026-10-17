#!/usr/bin/env bash
# Root opens only its own destination; the downloader reads its own source.
set -euo pipefail
[[ $# == 1 && $EUID == 0 && $(hostname -s) == silkboard ]] || exit 1
[[ $1 =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || exit 1
umask 077
snapshot=$(mktemp /var/lib/silkboard-input/bundle.XXXXXXXX)
trap 'rm -f -- "$snapshot"' EXIT
runuser -u silkfetch -- cat -- "/var/lib/silkboard-fetch/source-$1.bundle" > "$snapshot"
chown root:silkbuild "$snapshot"
chmod 0440 "$snapshot"
mv -fT -- "$snapshot" /var/lib/silkboard-input/source.bundle
