#!/usr/bin/env bash
# Build a separately admitted initial release; no GitHub or VM credentials.
set -euo pipefail
umask 022
fail() { echo "silkboard build: $*" >&2; exit 1; }
[[ $# == 3 ]] || fail 'usage: guest-build.sh BUNDLE TAG COMMIT'
[[ $(id -un) == silkbuild && $(hostname -s) == silkboard ]] ||
    fail 'run as silkbuild inside the production guest'
bundle=$1 tag=$2 commit=$3
[[ $tag =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ && $commit =~ ^[0-9a-f]{40}$ ]] ||
    fail 'stable release tag and full lowercase commit required'
[[ -f $bundle ]] || fail 'bundle not found'
base=/var/lib/silkboard-build
source=$base/source-$tag
output=$base/output-$tag
[[ ! -e $source && ! -e $output ]] || fail 'build paths exist; inspect before retry'
export CARGO_HOME=$base/cargo RUSTUP_HOME=$base/rustup CARGO_BUILD_JOBS=1
export PATH=/opt/node-v24.14.1/bin:$CARGO_HOME/bin:$PATH
git clone --no-checkout --no-local -- "$bundle" "$source"
cd "$source"
reference=refs/tags/$tag
[[ $(git rev-parse "$reference^{commit}") == "$commit" ]] || fail 'tag commit mismatch'
# Verify before checkout/build executes release-controlled code.
git -c gpg.ssh.allowedSignersFile=/etc/silkboard/release-signers \
    -c gpg.ssh.program=ssh-keygen verify-tag "$reference"
headers=$(git cat-file tag "$reference" | sed -n '1,3p')
[[ $headers == "$(printf 'object %s\ntype commit\ntag %s' "$commit" "$tag")" ]] ||
    fail 'signed tag name or direct target mismatch'
git checkout --detach "$commit"
cargo xtask release-check "$tag"
# Both stages are sequential and unprivileged, under the agreed fixed RAM cap.
npm ci --prefix frontend
npm run build --prefix frontend
SILKBOARD_RELEASE_TAG=$tag SILKBOARD_COMMIT=$commit \
    cargo build --locked --release -p silkboard-web
install -d "$output/frontend"
install -m 0755 target/release/silkboard-web "$output/silkboard-web"
cp -R -- frontend/dist/. "$output/frontend/"
printf '%s\n%s\n' "$tag" "$commit" > "$output/identity"
echo "Built $tag ($commit) in $output; operator activation is separate."
