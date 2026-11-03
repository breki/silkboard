#!/usr/bin/env bash
# Real Cargo and signed fixture tags; no registry/network/production access.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/../../.." && pwd)
work=$(mktemp -d)
cleanup() {
    status=$?
    trap - EXIT
    if [[ $status != 0 ]]; then
        for log in "$work"/v*.log; do [[ ! -f $log ]] || tail -35 "$log" >&2; done
    fi
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
real_cargo=$(rustup which cargo)
real_rustc=$(rustup which rustc)
# Nested fixture builds must not enter the parent coverage or shared outputs.
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTDOCFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
unset CARGO_LLVM_COV_TARGET_DIR CARGO_LLVM_COV LLVM_PROFILE_FILE CARGO_MAKEFLAGS MAKEFLAGS
export CARGO_TARGET_DIR=$work/generation-target
export fixture=$work
mkdir -m 0700 "$work/build"
mkdir -p "$work/build/cargo/bin" "$work/external/src" "$work/repo"
ln -s "$real_cargo" "$work/build/cargo/bin/cargo"
sed -e "s#base=/var/lib/silkboard-build#base=$work/build#" \
    -e "s#/etc/silkboard/release-signers#$work/signers#g" \
    "$root/deploy/vm/guest-build.sh" > "$work/helper"
grep -Fxq "base=$work/build" "$work/helper"
if grep -Fq -e /var/lib/silkboard-build -e /etc/silkboard/release-signers "$work/helper"; then
    echo 'Fixture isolation failed: production paths remain' >&2; exit 1
fi
id() { if [[ $1 == -un ]]; then echo silkbuild; else command id "$@"; fi; }
hostname() { echo silkboard; }
npm() {
    [[ $CARGO_BUILD_JOBS == 1 ]]
    if [[ $1 == run ]]; then
        mkdir -p frontend/dist
        cp frontend/source.txt frontend/dist/index.html
    fi
}
export -f id hostname npm
# Controlled compiler identities test Cargo invalidation without downloading Rust.
for identity in a b; do
    compiler_hash=$(printf '%040d' 0 | tr 0 "$identity")
    cat > "$work/compiler-$identity" <<PROXY
#!/usr/bin/env bash
if [[ \$1 == -vV ]]; then
    "$real_rustc" -vV | sed 's/^commit-hash:.*/commit-hash: $compiler_hash/'
else
    exec "$real_rustc" "\$@"
fi
PROXY
    chmod 0700 "$work/compiler-$identity"
done
export RUSTC=$work/compiler-a
cat > "$work/external/Cargo.toml" <<'TOML'
[package]
name="external-fixture"
version="0.1.0"
edition="2024"
[features]
extra=[]
TOML
cat > "$work/external/src/lib.rs" <<'RS'
pub fn value() -> String {
    format!("{}-{}", env!("CARGO_PKG_VERSION"), cfg!(feature="extra"))
}
RS
cd "$work/repo"
git init -q
ssh-keygen -q -t ed25519 -N '' -C fixture -f "$work/key"
printf 'fixture namespaces="git" %s\n' "$(cat "$work/key.pub")" > "$work/signers"
git config user.name Fixture
git config user.email fixture@example.invalid
git config gpg.format ssh
git config user.signingkey "$work/key"
git config commit.gpgsign true
git config tag.gpgsign true
mkdir -p .cargo web/src workdep/src xtask/src frontend
printf '/frontend/dist/\n/target/\n' > .gitignore
cat > .cargo/config.toml <<'TOML'
[alias]
xtask="run --package xtask --"
TOML
cat > Cargo.toml <<'TOML'
[workspace]
members=["web","workdep","xtask"]
resolver="2"
TOML
cat > workdep/Cargo.toml <<'TOML'
[package]
name="workdep"
version="0.1.0"
edition="2024"
TOML
printf 'pub fn value() -> &\x27static str { include_str!("../value.txt").trim() }\n' > workdep/src/lib.rs
cat > xtask/Cargo.toml <<'TOML'
[package]
name="xtask"
version="0.1.0"
edition="2024"
TOML
cat > xtask/src/main.rs <<'RS'
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert_eq!(root, std::env::current_dir().unwrap(), "stale workspace helper");
    assert_eq!(std::env::args().nth(1).as_deref(), Some("release-check"));
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain"]).output().unwrap();
    assert!(status.status.success());
    assert!(status.stdout.is_empty(), "candidate changed during build");
}
RS
cat > web/src/main.rs <<'RS'
fn main() {
    println!("{} {} {} {} {}", env!("CARGO_PKG_VERSION"), env!("SILKBOARD_RELEASE_TAG"),
        env!("SILKBOARD_COMMIT"), workdep::value(), external_fixture::value());
}
RS
make_tag() {
    version=$1 marker=$2 dependency=$3 features=$4
    printf '%s\n' "$marker" > workdep/value.txt
    printf '%s\n' "$marker" > frontend/source.txt
    cat > web/Cargo.toml <<TOML
[package]
name="silkboard-web"
version="$version"
edition="2024"
[dependencies]
workdep={path="../workdep"}
external-fixture={path="$work/external",version="=$dependency",features=[$features]}
TOML
    "$real_cargo" generate-lockfile --offline > /dev/null 2>&1
    git add .
    git commit -qm "fixture $version"
    git tag -s -m "fixture $version" "v$version"
    git bundle create "$work/bundle" --all > /dev/null 2>&1
    commit=$(git rev-parse HEAD)
}
assert_dependency() {
    node - "$1" "$2" "$3" <<'JS'
const fs=require('node:fs'), assert=require('node:assert/strict');
const line=fs.readFileSync(process.argv[2],'utf8').split('\n')
  .find(line=>line.startsWith('Build cache evidence: '));
assert(line,'missing Cargo cache evidence');
const evidence=JSON.parse(line.slice('Build cache evidence: '.length));
const key=process.argv[3]==='fresh'?'fresh_dependencies':'rebuilt_dependencies';
assert(evidence[key].some(id=>id.endsWith('#external-fixture@'+process.argv[4])),
  JSON.stringify(evidence));
JS
}
run_case() {
    version=$1 marker=$2 dependency=$3 features=$4 expected=$5
    make_tag "$version" "$marker" "$dependency" "$features"
    bash "$work/helper" "$work/bundle" "v$version" "$commit" > "$work/v$version.log" 2>&1
    [[ $("$work/build/output-v$version/silkboard-web") == "$version v$version $commit $marker $dependency-$([[ -z $features ]] && echo false || echo true)" ]]
    [[ $(cat "$work/build/output-v$version/frontend/index.html") == "$marker" ]]
    assert_dependency "$work/v$version.log" "$expected" "$dependency"
}
run_case 1.0.0 A 0.1.0 '' rebuilt
run_case 1.0.1 B 0.1.0 '' fresh
run_case 1.0.2 C 0.1.0 '"extra"' rebuilt
sed -i 's/version="0.1.0"/version="0.2.0"/' "$work/external/Cargo.toml"
run_case 1.0.3 D 0.2.0 '"extra"' rebuilt
export RUSTFLAGS='-C opt-level=1'
run_case 1.0.4 E 0.2.0 '"extra"' rebuilt
export RUSTC=$work/compiler-b
run_case 1.0.5 F 0.2.0 '"extra"' rebuilt
run_case 1.0.6 G 0.2.0 '"extra"' fresh
cache=$work/build/compile-cache-v1
flock "$cache/build.lock" rm -rf -- "$cache/target"
run_case 1.0.7 H 0.2.0 '"extra"' rebuilt
printf 'Real Cargo fixtures: signed candidates, dependency reuse, workspace freshness, features/versions/flags/compiler-proxy invalidation and cleared cache passed.\n'
