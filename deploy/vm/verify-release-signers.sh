#!/usr/bin/env bash
# Operator trust root: accept exactly the approved owner and isolated VM keys.
set -euo pipefail
fail() { echo 'silkboard guest: unexpected release trust set' >&2; exit 1; }
[[ $# == 1 && -f $1 && ! -L $1 ]] || fail
# Reject skipped malformed records, options, duplicate/extra keys and comments.
awk 'NF != 3 || $1 !~ /^[A-Za-z0-9_-]+$/ || $2 != "ssh-ed25519" {bad=1} END {exit (bad || NR != 2)}' "$1" || fail
actual=$(ssh-keygen -lf "$1" | awk '{print $2}' | LC_ALL=C sort) || fail
expected=$(printf '%s\n' \
    'SHA256:D+XaRkaUyW9uBQ8dMDp+XmZV6QH255p9g58auXRQwf8' \
    'SHA256:s/bOUAzymE3qEqSllzx4fLR+TNtaWMhpvLR1FrbSc9I' | LC_ALL=C sort)
[[ $actual == "$expected" ]] || fail
