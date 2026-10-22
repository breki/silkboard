#!/usr/bin/env bash
# Native Bombyx guest bootstrap. Run as the agent user, never on production.
set -euo pipefail
set +x
umask 077
fail() { echo "Silkboard VM provisioning: $*" >&2; exit 1; }
[[ $EUID != 0 && ${BOMBYX_PROJECT:-} == silkboard ]] || fail 'run as the Bombyx silkboard agent user'
root=$(git rev-parse --show-toplevel)
[[ $(realpath "$root") == "$HOME/silkboard" && $(pwd -P) == "$root" ]] || fail 'run from the guest checkout at ~/silkboard'
# shellcheck source=/dev/null
source /etc/os-release
[[ $ID == ubuntu && $VERSION_ID == 24.04 ]] || fail 'Ubuntu 24.04 required'
[[ -f $HOME/.ssh/bombyx-deploy-key && ! -L $HOME/.ssh/bombyx-deploy-key ]] || fail 'Bombyx deploy key missing'
sudo -n apt-get update
sudo -n env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev ca-certificates curl git openssh-client \
    jq ripgrep xz-utils unzip gnupg openssl gcc-aarch64-linux-gnu libc6-dev-arm64-cross
case $(uname -m) in
    x86_64)
        node_arch=x64; rust_arch=x86_64-unknown-linux-gnu
        node_sum=84d38715d449447117d05c3e71acd78daa49d5b1bfa8aacf610303920c3322be
        rust_sum=dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71 ;;
    aarch64)
        node_arch=arm64; rust_arch=aarch64-unknown-linux-gnu
        node_sum=71e427e28b78846f201d4d5ecc30cb13d1508ca099ef3871889a1256c7d6f67e
        rust_sum=15f6e4ce9f583b929c996c91562bad6d4454f3281de858b02cdfdef615fac433 ;;
    *) fail 'x86_64 or aarch64 guest required' ;;
esac
tools=$HOME/.local/share/silkboard-tools
mkdir -p "$tools" "$HOME/.local/bin"
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
runtime=$tools/node-v24.14.1
if [[ ! -x $runtime/bin/node ]] || [[ $("$runtime/bin/node" --version) != v24.14.1 ]]; then
    [[ ! -e $runtime && ! -L $runtime ]] || fail 'unexpected Node runtime; inspect before replacing it'
    curl -fSsL --retry 3 "https://nodejs.org/dist/v24.14.1/node-v24.14.1-linux-$node_arch.tar.xz" -o "$work/node.tar.xz"
    printf '%s  %s\n' "$node_sum" "$work/node.tar.xz" | sha256sum --check --status
    mkdir "$work/node"
    tar -xJf "$work/node.tar.xz" --strip-components=1 -C "$work/node"
    mv "$work/node" "$runtime"
fi
export PATH="$runtime/bin:$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
if [[ ! -x $HOME/.cargo/bin/rustup ]]; then
    curl -fSsL --retry 3 "https://static.rust-lang.org/rustup/archive/1.29.1/$rust_arch/rustup-init" -o "$work/rustup-init"
    printf '%s  %s\n' "$rust_sum" "$work/rustup-init" | sha256sum --check --status
    chmod 0700 "$work/rustup-init"
    "$work/rustup-init" -y --no-modify-path --profile minimal --default-toolchain 1.93.1
fi
rustup toolchain install 1.93.1 --profile minimal --component rustfmt,clippy,llvm-tools-preview
rustup override set 1.93.1
rustup target add --toolchain 1.93.1 aarch64-unknown-linux-gnu
install_gate() {
    local package=$1 version=$2 command=$3
    if [[ $(cargo "$command" --version 2>/dev/null | awk '{print $NF}') != "$version" ]]; then
        cargo install --locked --version "$version" "$package"
    fi
}
install_gate cargo-llvm-cov 0.9.0 llvm-cov
if ! code-dupes --version 2>/dev/null | grep -Fqx 'code-dupes 0.2.1'; then
    cargo install --locked --version 0.2.1 code-dupes
fi
install_gate cargo-deny 0.20.2 deny
install_gate cargo-audit 0.22.2 audit
agents=$tools/agents
if [[ ! -x $agents/bin/codex ]] || [[ $("$agents/bin/codex" --version) != 'codex-cli 0.162.1' ]] || \
   [[ ! -x $agents/bin/claude ]] || [[ $("$agents/bin/claude" --version) != '2.1.293 (Claude Code)' ]]; then
    npm install --global --prefix "$agents" @openai/codex@0.162.1 @anthropic-ai/claude-code@2.1.293
fi
link_tool() {
    local source=$1 dest=$HOME/.local/bin/$2
    if [[ -e $dest || -L $dest ]]; then
        [[ -L $dest && $(readlink "$dest") == "$source" ]] || fail "refusing to replace existing $2"
    else
        ln -s "$source" "$dest"
    fi
}
for tool in node npm npx; do link_tool "$runtime/bin/$tool" "$tool"; done
for tool in cargo rustc rustup rustfmt cargo-clippy cargo-llvm-cov cargo-deny cargo-audit code-dupes; do link_tool "$HOME/.cargo/bin/$tool" "$tool"; done
for tool in codex claude; do link_tool "$agents/bin/$tool" "$tool"; done
npm ci --prefix frontend
(cd frontend && npm exec -- playwright install --with-deps chromium)
CARGO_TARGET_DIR="$root/target" cargo build --locked --release -p silkboard-publish
# Install from an owned fresh temporary file; readers see the complete executable.
install -m 0755 target/release/silkboard-publish "$work/silkboard-publish"
publisher_temp=$(mktemp "$HOME/.local/bin/.silkboard-publish.XXXXXX")
trap 'rm -rf -- "$work"; rm -f -- "$publisher_temp"' EXIT
install -m 0755 "$work/silkboard-publish" "$publisher_temp"
mv -T "$publisher_temp" "$HOME/.local/bin/silkboard-publish"
"$runtime/bin/node" "$root/.bombyx/configure-cli.mjs" "$root"
bash "$root/.bombyx/refresh-secrets.sh"
echo 'Silkboard VM tools, Git signing and private publisher configuration installed.'
echo 'Agent authentication, native hook trust, endpoint reachability and full validation still require verification.'
