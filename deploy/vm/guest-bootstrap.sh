#!/usr/bin/env bash
# Root bootstrap before Cargo exists; does not authenticate Tailscale or deploy.
set -euo pipefail
umask 022
fail() { echo "silkboard guest: $*" >&2; exit 1; }
[[ $# == 2 ]] || fail 'usage: guest-bootstrap.sh SIGNERS_FILE SERVICE_FILE'
[[ $EUID == 0 && $(hostname -s) == silkboard && $(uname -m) == aarch64 ]] ||
    fail 'run as root in the production silkboard ARM64 guest'
# shellcheck source=/dev/null
source /etc/os-release
[[ $ID == debian && $VERSION_ID == 13 ]] || fail 'Debian 13 required'
cloud-init status --wait
for user in silkadmin silkbuild silkboard; do id "$user" >/dev/null; done
[[ $(ssh-keygen -lf "$1" | awk '{print $2}') == SHA256:D+XaRkaUyW9uBQ8dMDp+XmZV6QH255p9g58auXRQwf8 ]] ||
    fail 'unexpected release trust key'
install -d -m 0755 /etc/silkboard /opt/silkboard/releases
install -m 0644 "$1" /etc/silkboard/release-signers
install -m 0644 "$2" /etc/systemd/system/silkboard.service
install -d -m 0700 -o silkbuild -g silkbuild /var/lib/silkboard-build
curl -fsSL --retry 3 https://pkgs.tailscale.com/stable/debian/trixie.noarmor.gpg \
    -o /usr/share/keyrings/tailscale-archive-keyring.gpg
curl -fsSL --retry 3 https://pkgs.tailscale.com/stable/debian/trixie.tailscale-keyring.list \
    -o /etc/apt/sources.list.d/tailscale.list
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends tailscale
systemctl enable --now tailscaled
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
curl -fsSL --retry 3 https://nodejs.org/dist/v24.14.1/node-v24.14.1-linux-arm64.tar.xz \
    -o "$work/node.tar.xz"
printf '%s  %s\n' 71e427e28b78846f201d4d5ecc30cb13d1508ca099ef3871889a1256c7d6f67e \
    "$work/node.tar.xz" | sha256sum --check --status
install -d -m 0755 /opt/node-v24.14.1
tar -xJf "$work/node.tar.xz" --strip-components=1 -C /opt/node-v24.14.1
curl -fsSL --retry 3 https://static.rust-lang.org/rustup/archive/1.29.1/aarch64-unknown-linux-gnu/rustup-init \
    -o "$work/rustup-init"
printf '%s  %s\n' 15f6e4ce9f583b929c996c91562bad6d4454f3281de858b02cdfdef615fac433 \
    "$work/rustup-init" | sha256sum --check --status
install -m 0755 "$work/rustup-init" /var/lib/silkboard-build/rustup-init
runuser -u silkbuild -- env \
    CARGO_HOME=/var/lib/silkboard-build/cargo RUSTUP_HOME=/var/lib/silkboard-build/rustup \
    /var/lib/silkboard-build/rustup-init -y --no-modify-path --profile minimal \
    --default-toolchain 1.93.1
systemctl daemon-reload
echo 'Bootstrap complete. Tailscale still needs operator login; no app activated.'
