#!/usr/bin/env bash
# Execute the real helper in an owned filesystem with fake compiler/Git I/O.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/../../.." && pwd)
work=$(mktemp -d)
children=()
cleanup() {
    local status=$?
    trap - EXIT
    for pid in "${children[@]}"; do kill -- -"$pid" 2>/dev/null || true; done
    for pid in "${children[@]}"; do wait "$pid" 2>/dev/null || true; done
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
export fixture=$work
mkdir -m 0700 "$work/build"
printf bundle > "$work/bundle"
sed -e "s#base=/var/lib/silkboard-build#base=$work/build#" \
    -e "s#/etc/silkboard/release-signers#$work/signers#g" \
    "$root/deploy/vm/guest-build.sh" > "$work/helper"
grep -Fxq "base=$work/build" "$work/helper"
if grep -Fq -e /var/lib/silkboard-build -e /etc/silkboard/release-signers "$work/helper"; then
    echo 'Fixture isolation failed: production paths remain' >&2; exit 1
fi
id() { if [[ $1 == -un ]]; then echo "${BAD_ACCOUNT:-silkbuild}"; else command id "$@"; fi; }
hostname() { echo silkboard; }
git() {
    printf 'git %s\n' "$*" >> "$fixture/calls"
    case $1 in
        clone)
            mkdir -p "${@: -1}/frontend" "${@: -1}/web" "${@: -1}/xtask"
            touch "${@: -1}/web/Cargo.toml" "${@: -1}/xtask/Cargo.toml"
            ;;
        rev-parse) printf '%s\n' "$FIXTURE_COMMIT" ;;
        -c) [[ ${BAD_SIGNATURE:-0} == 0 ]] ;;
        cat-file) printf 'object %s\ntype commit\ntag %s\n' "$FIXTURE_COMMIT" "$FIXTURE_TAG" ;;
        checkout) : ;;
        *) echo "unexpected git $*" >&2; return 1 ;;
    esac
}
npm() {
    [[ $CARGO_BUILD_JOBS == 1 ]]
    if [[ $1 == run ]]; then
        mkdir -p frontend/dist
        printf '%s\n' "$FIXTURE_TAG" > frontend/dist/index.html
    fi
}
cargo() (
    set -euo pipefail
    printf 'cargo %s\n' "$*" >> "$fixture/calls"
    target=${CARGO_TARGET_DIR:-$PWD/target}
    case $1 in
        metadata)
            node -e 'console.log(JSON.stringify({workspace_root:process.cwd(),workspace_members:["web-id","xtask-id"],packages:[{id:"web-id",name:"silkboard-web",version:process.env.BAD_VERSION==="1"?"0.0.0":process.env.FIXTURE_TAG.slice(1),manifest_path:process.cwd()+"/web/Cargo.toml"},{id:"xtask-id",name:"xtask",manifest_path:process.cwd()+"/xtask/Cargo.toml"}]}))'
            ;;
        clean)
            [[ " $* " == *' --package silkboard-web '* && " $* " == *' --package xtask '* ]]
            [[ ${BAD_CLEAN:-0} == 0 ]] || exit 8
            rm -f -- "$target/release/silkboard-web" "$target/debug/xtask"
            ;;
        xtask)
            [[ ! -e $target/debug/xtask ]] || exit 44
            [[ ${BAD_RELEASE_CHECK:-0} == 0 ]] || exit 9
            ;;
        build)
            mkdir -p "$target/release/deps"
            if [[ ! -f $target/release/deps/external.a ]]; then
                printf compiled >> "$fixture/dependency-builds"
                printf dependency > "$target/release/deps/external.a"
            fi
            if [[ ${HOLD_BUILD:-0} == 1 ]]; then
                touch "$fixture/entered"
                while [[ ! -f $fixture/release-build ]]; do sleep 0.02; done
            fi
            if [[ ${BAD_BUILD:-0} == 1 ]]; then
                touch "$target/over-budget"
                printf '{"reason":"compiler-message","message":{"rendered":"fixture compile error"}}\n'
                exit 7
            fi
            printf '#!/bin/sh\nprintf "%%s\\n" "%s %s"\n' "$FIXTURE_TAG" "$FIXTURE_COMMIT" > "$target/release/silkboard-web"
            chmod 0755 "$target/release/silkboard-web"
            if [[ " $* " == *' --message-format=json '* ]]; then
                printf 'fixture procedural macro stdout\n'
                BINARY_PATH="$target/release/silkboard-web" node -e 'console.log(JSON.stringify({reason:"compiler-artifact",package_id:"web-id",target:{name:"silkboard-web",kind:["bin"]},executable:process.env.BAD_ARTIFACT_PATH==="1"?"/outside/backend":process.env.BINARY_PATH,fresh:process.env.BAD_FRESH==="1"}))'
            fi
            ;;
        *) echo "unexpected cargo $*" >&2; exit 1 ;;
    esac
)
du() {
    if [[ -f ${@: -1}/over-budget ]]; then printf '4194305\t%s\n' "${@: -1}";
    else command du "$@"; fi
}
export -f id hostname git npm cargo du
run_build() {
    local tag=$1 commit=$2
    FIXTURE_TAG=$tag FIXTURE_COMMIT=$commit bash "$work/helper" "$work/bundle" "$tag" "$commit" > "$work/$tag.log" 2>&1
}
a=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
b=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
run_build v1.0.0 "$a"
run_build v1.0.1 "$b"
[[ $(cat "$work/dependency-builds") == compiled ]] || {
    echo 'External dependency was compiled more than once across release paths' >&2; exit 1;
}
cache=$work/build/compile-cache-v1
[[ $(stat -c %a "$cache") == 700 && $(stat -c %a "$cache/target") == 700 ]]
grep -q 'fixture procedural macro stdout' "$work/v1.0.1.log"
[[ $("$work/build/output-v1.0.1/silkboard-web") == "v1.0.1 $b" ]]
[[ $(cat "$work/build/output-v1.0.1/frontend/index.html") == v1.0.1 ]]
[[ $(cat "$work/build/output-v1.0.0/identity") == "$(printf 'v1.0.0\n%s' "$a")" ]]
# Every candidate clears old workspace binaries before invoking its xtask.
mkdir -p "$cache/target/debug"
printf stale > "$cache/target/debug/xtask"
run_build v1.0.2 "$a"
# Failed signature must not invoke Cargo or evict retained dependencies.
touch "$cache/target/over-budget"
count=$(grep -c '^cargo ' "$work/calls")
if BAD_SIGNATURE=1 run_build v1.0.3 "$a"; then exit 1; fi
[[ $(grep -c '^cargo ' "$work/calls") == "$count" && -f $cache/target/over-budget ]]
rm "$cache/target/over-budget"
# Refuse incompatible metadata/fresh outputs and preserve command failure codes.
if BAD_FRESH=1 run_build v1.0.4 "$a"; then exit 1; fi
[[ ! -e $work/build/output-v1.0.4 ]]
if BAD_CLEAN=1 run_build v1.0.5 "$a"; then exit 1; else [[ $? == 8 ]]; fi
if BAD_RELEASE_CHECK=1 run_build v1.0.6 "$a"; then exit 1; else [[ $? == 9 ]]; fi
lock_inode=$(stat -c %i "$cache/build.lock")
if BAD_BUILD=1 run_build v1.0.7 "$a"; then exit 1; else [[ $? == 7 ]]; fi
grep -q 'fixture compile error' "$work/v1.0.7.log"
[[ ! -e $cache/target/over-budget && ! -e $cache/target/release/deps/external.a ]]
[[ $(stat -c %i "$cache/build.lock") == "$lock_inode" ]]
run_build v1.0.8 "$b"
[[ $(cat "$work/dependency-builds") == compiledcompiled ]]
# Retention pruning happens only under the stable whole-command lock.
FIXTURE_TAG=v1.0.9 FIXTURE_COMMIT=$a HOLD_BUILD=1 setsid bash "$work/helper" "$work/bundle" v1.0.9 "$a" > "$work/v1.0.9.log" 2>&1 &
first=$!
children+=("$first")
for ((i=0;i<200;i++)); do [[ ! -e $work/entered ]] || break; sleep 0.02; done
[[ -f $work/entered ]]
FIXTURE_TAG=v1.0.10 FIXTURE_COMMIT=$b setsid bash "$work/helper" "$work/bundle" v1.0.10 "$b" > "$work/v1.0.10.log" 2>&1 &
second=$!
children+=("$second")
sleep 0.15
[[ ! -e $work/build/source-v1.0.10 ]]
touch "$work/release-build"
wait "$first"
wait "$second"
# No following symlink roots or changing an existing public cache's mode.
mv "$cache/target" "$work/target-backup"
mkdir "$work/victim"
printf keep > "$work/victim/keep"
ln -s "$work/victim" "$cache/target"
if run_build v1.0.11 "$a"; then exit 1; fi
[[ $(cat "$work/victim/keep") == keep ]]
rm "$cache/target"
mv "$work/target-backup" "$cache/target"
chmod 0755 "$cache"
if run_build v1.0.12 "$a"; then exit 1; fi
[[ $(stat -c %a "$cache") == 755 ]]
chmod 0700 "$cache"
if BAD_VERSION=1 run_build v1.0.14 "$a"; then exit 1; fi
if BAD_ARTIFACT_PATH=1 run_build v1.0.15 "$a"; then exit 1; fi
[[ ! -e $work/build/output-v1.0.14 && ! -e $work/build/output-v1.0.15 ]]
if BAD_ACCOUNT=other run_build v1.0.13 "$a"; then exit 1; fi
if bash "$work/helper" "$work/bundle" 'v1.0.0/../bad' "$a"; then exit 1; fi
printf 'Guest cache contracts: reuse, freshness, failures, locking, retention and privacy passed.\n'
