#!/usr/bin/env bash
# Use Codex's standalone lifecycle; never change private agent state or npm policy.
set -euo pipefail
set +x
umask 077
fail() { echo "Silkboard Codex installation: $*" >&2; exit 1; }
[[ $EUID != 0 && ${BOMBYX_PROJECT:-} == silkboard ]] || fail 'run as the Bombyx silkboard agent user'
[[ ${CODEX_HOME:-$HOME/.codex} == "$HOME/.codex" ]] || fail 'custom CODEX_HOME requires explicit migration; leaving it unchanged'
bin=$HOME/.local/bin
dest=$bin/codex
tools=$HOME/.local/share/silkboard-tools
agents=$tools/agents
runtime=$tools/node-v24.14.1
current=$HOME/.codex/packages/standalone/current

is_standalone_link() {
    [[ -L $1 ]] || return 1
    local target
    target=$(readlink "$1")
    [[ $target == "$current/bin/codex" || $target == "$current/codex" ]]
}
is_npm_link() {
    local prefix=$1 target
    [[ -L $prefix/bin/codex ]] || return 1
    target=$(readlink "$prefix/bin/codex")
    [[ $target == ../lib/node_modules/@openai/codex/bin/codex.js || \
       $target == "$prefix/lib/node_modules/@openai/codex/bin/codex.js" ]]
}
check_command() {
    local command=$1
    [[ -e $command || -L $command ]] || return 0
    if [[ $command == "$dest" ]]; then
        is_standalone_link "$command" && return 0
        [[ -L $command && $(readlink "$command") == "$agents/bin/codex" ]] && return 0
    fi
    for prefix in "$agents" "$runtime"; do
        if [[ $command == "$prefix/bin/codex" ]] && is_npm_link "$prefix"; then return 0; fi
    done
    fail "refusing unrelated Codex command at $command; inspect before replacing it"
}

# Include the known install locations even when they are absent from PATH.
for command in "$dest" "$agents/bin/codex" "$runtime/bin/codex"; do check_command "$command"; done
IFS=: read -r -a search_dirs <<< "$PATH"
for directory in "${search_dirs[@]}"; do check_command "${directory:-.}/codex"; done

if is_standalone_link "$dest"; then
    [[ -x $dest ]] || fail 'standalone launcher is broken; inspect the selected release'
else
    work=$(mktemp -d)
    trap 'rm -rf -- "$work"' EXIT
    curl -fSsL --connect-timeout 10 --max-time 60 https://chatgpt.com/codex/install.sh -o "$work/install.sh"
    # Isolate installer controls from caller overrides (daemon-only, release,
    # deferred selection, alternate home). Defaults retain native self-updates.
    env -i HOME="$HOME" PATH="$PATH" CODEX_HOME="$HOME/.codex" \
        CODEX_INSTALL_DIR="$bin" CODEX_NON_INTERACTIVE=1 CODEX_RELEASE=latest \
        sh "$work/install.sh"
fi
is_standalone_link "$dest" && [[ -x $dest ]] || fail 'installer did not select the expected standalone launcher'
version=$("$dest" --version)
[[ $version =~ ^codex-cli\ [0-9]+\.[0-9]+\.[0-9]+ ]] || fail 'standalone command did not report a Codex version'

# Retire only recognized npm launchers after a successful cutover. Package
# contents remain inert; no npm runtime or package lifecycle scripts are needed.
for prefix in "$agents" "$runtime"; do
    if is_npm_link "$prefix"; then unlink "$prefix/bin/codex"; fi
done
export PATH="$bin:$PATH"
hash -r
[[ $(readlink -f "$(command -v codex)") == "$(readlink -f "$dest")" ]] || fail 'PATH resolves to another Codex installation'
[[ $(codex --version) == "$version" ]] || fail 'Codex version differs through PATH'
echo "Silkboard Codex installation: $version (standalone)."
