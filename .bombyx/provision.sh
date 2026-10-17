#!/usr/bin/env bash
# Bombyx runs this as the agent user from the guest's project clone.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
[[ $(id -u) != 0 ]] || { echo 'Run provisioning as the Bombyx agent user.' >&2; exit 1; }
[[ -n ${BOMBYX_VM_HOST:-} ]] || {
    echo 'This provisioner is for a Bombyx development VM.' >&2
    exit 1
}
sudo -n apt-get update
sudo -n apt-get install -y --no-install-recommends \
    build-essential ca-certificates curl git gh jq openssh-client pkg-config \
    libssl-dev ripgrep tmux xz-utils shellcheck bubblewrap
bash "$root/.bombyx/install-tools.sh"
bash "$root/.bombyx/setup.sh"
echo 'Development VM provisioned. Open a new bombyx shell, then log in to Codex and Claude Code.'
