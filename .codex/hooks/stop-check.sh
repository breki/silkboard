#!/usr/bin/env bash
# Adapted from Bombyx's Stop hook; see docs/ai-agents/provenance.md.
set -euo pipefail

fail() {
  printf 'Stop validation unavailable: %s\n' "$1" >&2
  exit 2
}

# Without the parser we cannot read the recursion flag. Do not request a
# continuation that would fail again before reaching that flag.
if ! command -v jq >/dev/null; then
  printf '%s\n' '{"systemMessage":"Stop validation unavailable: jq is required to parse the hook payload. Checks did not run."}'
  exit 0
fi
input=$(cat)
if ! jq -e 'type == "object"' >/dev/null 2>&1 <<<"$input"; then
  fail 'expected a JSON object on stdin.'
fi
if jq -e '.stop_hook_active == true' >/dev/null <<<"$input"; then
  exit 0
fi
if ! checkout=$(jq -er '.cwd | select(type == "string" and length > 0)' <<<"$input"); then
  fail 'the payload must contain a nonempty cwd string.'
fi
command -v git >/dev/null || fail 'Git is required.'
if ! root=$(git -C "$checkout" rev-parse --show-toplevel 2>/dev/null); then
  fail 'cwd must identify a Git worktree.'
fi
cd "$root"

# Include staged, unstaged, and untracked files, including an unborn HEAD.
if ! changed_rs=$(
  git diff --name-only -- '*.rs' &&
    git diff --cached --name-only -- '*.rs' &&
    git ls-files --others --exclude-standard -- '*.rs'
); then
  fail 'could not inspect changed Rust files.'
fi
[[ -n "$changed_rs" ]] || exit 0

command -v cargo >/dev/null || fail 'Cargo is required for changed Rust files.'
[[ -f Cargo.toml ]] || fail 'Cargo.toml is missing; the Cargo workspace is pending.'

run_stage() {
  local label="$1" output
  shift
  if ! output=$("$@" 2>&1); then
    printf '=== %s failed or unavailable ===\n%s\n' "$label" "$output" >&2
    exit 2
  fi
}

run_stage 'cargo fmt --check' cargo fmt --all -- --check
run_stage 'cargo xtask clippy' cargo xtask clippy
run_stage 'cargo xtask doc' cargo xtask doc
run_stage 'cargo xtask test' cargo xtask test
