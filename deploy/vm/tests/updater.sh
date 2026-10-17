#!/usr/bin/env bash
# Exercise actual shell boundaries with temporary artifacts and fake account/I/O.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/../../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
export fixture=$work
fixture_tag=v0.2.0 fixture_commit=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
source "$root/deploy/vm/guest-stage.sh"
runuser() { shift 3; "$@"; }
mkdir -p "$work/source/frontend/assets"
printf '%s\n%s\n' "$fixture_tag" "$fixture_commit" > "$work/source/identity"
printf binary > "$work/source/silkboard-web"
printf index > "$work/source/frontend/index.html"
printf asset > "$work/source/frontend/assets/a space.js"
chmod 4755 "$work/source/silkboard-web"
ln -s /etc/passwd "$work/source/frontend/ignored-link"
mkfifo "$work/source/frontend/ignored-fifo"
snapshot_artifacts "$work/source" "$work/staged" "$fixture_tag" "$fixture_commit"
[[ $(stat -c %a "$work/staged/silkboard-web") == 755 ]]
[[ $(stat -c %a "$work/staged/frontend/index.html") == 644 ]]
[[ -f $work/staged/frontend/assets/'a space.js' ]]
[[ ! -e $work/staged/frontend/ignored-link && ! -e $work/staged/frontend/ignored-fifo ]]
if snapshot_artifacts "$work/source" "$work/staged" "$fixture_tag" "$fixture_commit"; then exit 1; fi
printf '%s\n%s\n' v0.3.0 "$fixture_commit" > "$work/source/identity"
if snapshot_artifacts "$work/source" "$work/bad" "$fixture_tag" "$fixture_commit"; then exit 1; fi

# Only fake token text is used; no live credential is read by this test.
id() { echo silkfetch; }
cat() { printf github_pat_fixture; }
curl() { printf '%s\n' "$@" > "$fixture/argv"; /bin/cat > "$fixture/config"; }
export -f id cat curl
bash "$root/deploy/vm/github-read.sh" 'actions/workflows/387356305' > "$work/result"
grep -q 'github_pat_fixture' "$work/config"
if grep -q 'github_pat_fixture' "$work/argv"; then exit 1; fi
grep -q '^https://api.github.com/repos/breki/silkboard/actions/workflows/387356305$' "$work/argv"
if bash "$root/deploy/vm/github-read.sh" '../outside'; then exit 1; fi
printf 'protocol=https\nhost=github.com\n\n' |
    bash "$root/deploy/vm/git-credential.sh" get > "$work/credential"
grep -q '^username=x-access-token$' "$work/credential"
if printf 'protocol=https\nhost=example.com\n\n' |
    bash "$root/deploy/vm/git-credential.sh" get > "$work/rejected"; then exit 1; fi
[[ ! -s $work/rejected ]]
echo 'Updater shell boundaries: snapshot modes/identity and secret transport passed.'

# Both Toolkit and compatibility commands route through the same installed lock.
source "$root/deploy/vm/deployctl.sh"
deployctl_exec() { printf '%s\n' "$@" > "$fixture/toolkit-args"; }
for operation in poll status; do
    deployctl_main "$operation" silkboard
    [[ $(/bin/cat "$fixture/toolkit-args") == "$operation" ]]
done
for operation in inspect retry; do
    deployctl_main "$operation" silkboard v0.2.0
    [[ $(/bin/cat "$fixture/toolkit-args") == "$(printf '%s\n%s' "$operation" v0.2.0)" ]]
done
if deployctl_main poll other; then exit 1; fi
if deployctl_main --exec silkboard; then exit 1; fi
if deployctl_main poll silkboard extra; then exit 1; fi
if deployctl_main retry silkboard; then exit 1; fi
if deployctl_main; then exit 1; fi
# Generic evidence transport still restricts the URL path and hides the token.
bash "$root/deploy/vm/github-read.sh" example/site 'actions/workflows/57' > "$work/result"
grep -q '^https://api.github.com/repos/example/site/actions/workflows/57$' "$work/argv"
for repository in '../site' 'example/..' 'example/site?token=x' 'example/site/extra'; do
    if bash "$root/deploy/vm/github-read.sh" "$repository" 'actions/workflows/57'; then exit 1; fi
done
if bash "$root/deploy/vm/github-read.sh" example/site 'actions/../../user'; then
    echo 'API helper accepted path traversal' >&2; exit 1
fi
