#!/usr/bin/env bash
# Run from the native development VM after tool installation.
set -euo pipefail
[[ $(id -u) != 0 ]] || { echo 'Run project setup as the agent user.' >&2; exit 1; }
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
# shellcheck disable=SC1091 # Generated in the agent home by install-tools.sh.
source "$HOME/.config/silkboard/dev-env.sh"
cd "$root"
npm ci --prefix frontend
(cd frontend && npm exec -- playwright install --with-deps chromium)
cargo fetch --locked
codex --version
claude --version
