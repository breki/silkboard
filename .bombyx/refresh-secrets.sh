#!/usr/bin/env bash
# Bombyx starts this hook with a bare environment; never source its secret file.
set -euo pipefail
set +x
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
exec "$HOME/.local/share/silkboard-tools/node-v24.14.1/bin/node" \
    "$root/.bombyx/refresh-cli.mjs"
