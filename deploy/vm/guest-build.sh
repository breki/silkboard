#!/usr/bin/env bash
# Installed bootstrap: verify source, then reuse only compatible dependencies.
# Cache preparation precedes candidate xtask, which may itself be stale.
set -euo pipefail
umask 077
fail() { echo "silkboard build: $*" >&2; exit 1; }
private_directory() {
    [[ -d $1 && ! -L $1 && -O $1 && $(stat -c %a "$1") == 700 ]]
}
prepare_directory() {
    [[ ! -L $1 ]] || fail 'symlink cache directory'
    mkdir -p -m 0700 -- "$1"
    private_directory "$1" || fail 'cache requires owned private directories (0700)'
}
prune_cache() {
    private_directory "$cache" && private_directory "$target" || return 1
    local allocated
    allocated=$(du -sk -- "$target") || return 1
    allocated=${allocated%%[[:space:]]*}
    [[ $allocated =~ ^[0-9]+$ ]] || return 1
    if ((allocated > 4194304)); then
        # Never remove the cache root/lock or candidate source/output paths.
        rm -rf -- "$target" || return 1
        mkdir -m 0700 -- "$target" || return 1
        echo "Build cache: cleared $allocated KiB above 4194304 KiB retention limit." >&2
    else
        echo "Build cache: retained $allocated KiB (limit 4194304 KiB)." >&2
    fi
}
cleanup() {
    local status=$?
    trap - EXIT
    [[ -z ${metadata_file:-} ]] || rm -f -- "$metadata_file"
    [[ -z ${artifacts_file:-} ]] || rm -f -- "$artifacts_file"
    if [[ ${cache_admitted:-0} == 1 ]] && ! prune_cache; then
        echo 'Build cache cleanup failed; inspect before retry.' >&2
        [[ $status != 0 ]] || status=1
    fi
    exit "$status"
}
[[ $# == 3 ]] || fail 'usage: guest-build.sh BUNDLE TAG COMMIT'
[[ $(id -un) == silkbuild && $(hostname -s) == silkboard ]] ||
    fail 'run as silkbuild inside the production guest'
bundle=$1 tag=$2 commit=$3
[[ $tag =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ && $commit =~ ^[0-9a-f]{40}$ ]] ||
    fail 'stable release tag and full lowercase commit required'
[[ -f $bundle ]] || fail 'bundle not found'
base=/var/lib/silkboard-build
private_directory "$base" || fail 'builder root requires owned mode 0700'
cache=$base/compile-cache-v1
target=$cache/target
prepare_directory "$cache"
[[ ! -L $cache/build.lock ]] || fail 'symlink cache lock'
if [[ -e $cache/build.lock ]]; then
    [[ -f $cache/build.lock && -O $cache/build.lock && $(stat -c %a "$cache/build.lock") == 600 && $(stat -c %h "$cache/build.lock") == 1 ]] ||
        fail 'cache lock must be an owned private regular file'
fi
exec 9>>"$cache/build.lock"
flock --exclusive 9
trap cleanup EXIT
source=$base/source-$tag
output=$base/output-$tag
[[ ! -e $source && ! -L $source && ! -e $output && ! -L $output ]] ||
    fail 'build paths exist; inspect before retry'
export CARGO_HOME=$base/cargo RUSTUP_HOME=$base/rustup CARGO_BUILD_JOBS=1
export PATH=/opt/node-v24.14.1/bin:$CARGO_HOME/bin:$PATH
git clone --no-checkout --no-local -- "$bundle" "$source"
cd "$source"
reference=refs/tags/$tag
[[ $(git rev-parse "$reference^{commit}") == "$commit" ]] || fail 'tag commit mismatch'
# Keep trust independently installed; never read candidate-provided signers.
git -c gpg.ssh.allowedSignersFile=/etc/silkboard/release-signers \
    -c gpg.ssh.program=ssh-keygen verify-tag "$reference"
headers=$(git cat-file tag "$reference" | sed -n '1,3p')
[[ $headers == "$(printf 'object %s\ntype commit\ntag %s' "$commit" "$tag")" ]] ||
    fail 'signed tag name or direct target mismatch'
git checkout --detach "$commit"
prepare_directory "$target"
cache_admitted=1
prune_cache
export CARGO_TARGET_DIR=$target
metadata_file=$(mktemp "$target/.silkboard-metadata.XXXXXX")
artifacts_file=$(mktemp "$target/.silkboard-artifacts.XXXXXX")
cargo metadata --locked --no-deps --format-version 1 > "$metadata_file"
packages=$(node - "$metadata_file" "$source" "$tag" <<'JS'
const fs = require('node:fs'), path = require('node:path');
const m = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (m.workspace_root !== process.argv[3]) throw Error('wrong workspace root');
const members = m.packages.filter(p => m.workspace_members.includes(p.id));
if (!members.length || members.filter(p => p.name === 'silkboard-web').length !== 1)
  throw Error('expected current workspace and one backend package');
const web = members.find(p => p.name === 'silkboard-web');
if (web.version !== process.argv[4].slice(1)) throw Error('backend version/tag mismatch');
for (const p of members) {
  const relative = path.relative(process.argv[3], fs.realpathSync(p.manifest_path));
  if (relative === '..' || relative.startsWith('../') || path.isAbsolute(relative))
    throw Error('workspace manifest outside candidate');
}
console.log(members.map(p => p.name).join('\n'));
JS
)
clean_args=()
while IFS= read -r package; do clean_args+=(--package "$package"); done <<< "$packages"
# This must happen before any workspace executable, including release-check.
cargo clean "${clean_args[@]}"
cargo xtask release-check "$tag"
# Both stages remain sequential and unprivileged under the fixed guest budget.
npm ci --prefix frontend
npm run build --prefix frontend
if SILKBOARD_RELEASE_TAG=$tag SILKBOARD_COMMIT=$commit \
    cargo build --locked --release -p silkboard-web --message-format=json > "$artifacts_file"; then
    :
else
    status=$?
    cat -- "$artifacts_file" >&2
    exit "$status"
fi
binary=$(node - "$metadata_file" "$artifacts_file" "$target" <<'JS'
const fs = require('node:fs'), path = require('node:path');
const m = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const members = new Set(m.workspace_members);
const web = m.packages.find(p => members.has(p.id) && p.name === 'silkboard-web');
const lines = fs.readFileSync(process.argv[3], 'utf8').split('\n');
for (const line of lines) if (line.trim() && !line.startsWith('{')) console.error(line);
const records = lines.filter(line => line.startsWith('{')).map(line => JSON.parse(line));
for (const r of records)
  if (r.reason === 'compiler-message' && r.message?.rendered) console.error(r.message.rendered);
const artifacts = records.filter(a => a.reason === 'compiler-artifact');
if (artifacts.some(a => members.has(a.package_id) && a.fresh !== false))
  throw Error('workspace artifacts must rebuild for this candidate');
const matches = artifacts.filter(a => a.package_id === web.id &&
  a.target.name === 'silkboard-web' && a.target.kind.includes('bin') && a.executable);
if (matches.length !== 1) throw Error('expected one current backend artifact');
const expected = path.join(process.argv[4], 'release', 'silkboard-web');
if (matches[0].executable !== expected || fs.realpathSync(expected) !== expected ||
    !fs.lstatSync(expected).isFile()) throw Error('backend output path mismatch');
const ids = fresh => [...new Set(artifacts.filter(a =>
  !members.has(a.package_id) && a.fresh === fresh).map(a => a.package_id))];
console.error('Build cache evidence: ' + JSON.stringify({
  fresh_dependencies: ids(true), rebuilt_dependencies: ids(false)
}));
console.log(expected);
JS
)
# Recheck exact source identity/cleanliness after dependency and build scripts.
cargo xtask release-check "$tag"
install -d "$output/frontend"
install -m 0755 "$binary" "$output/silkboard-web"
cp -R -- frontend/dist/. "$output/frontend/"
printf '%s\n%s\n' "$tag" "$commit" > "$output/identity"
echo "Built $tag ($commit) in $output; operator activation is separate."
