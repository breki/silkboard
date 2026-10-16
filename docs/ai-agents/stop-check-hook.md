# Codex Stop validation hook

[hooks.json](../../.codex/hooks.json) registers the
[Stop script](../../.codex/hooks/stop-check.sh), adapted from Bombyx, alongside
the existing [Kozmotic Git summary](git-status-hook.md). The hook requires
Bash, Git, and jq. Its command resolves the script from the current Git
checkout; the payload's `cwd` selects the worktree to validate.

The script checks staged, unstaged, and untracked Rust files, including
deletions. With no changed Rust files it exits quietly. Otherwise it runs
these stages sequentially
from the worktree root and stops at the first failure:

1. `cargo fmt --all -- --check`
2. `cargo xtask clippy`
3. `cargo xtask doc`
4. `cargo xtask test`

Cargo, the workspace, and those xtask commands must exist before these gates
can pass. Missing prerequisites are reported as unavailable, never as passing
validation. Silkboard's Cargo workspace and xtask are currently pending; see
[tooling status](tooling.md). A missing or failing Cargo stage returns exit
code 2 with the stage and captured diagnostics. Codex uses that feedback to
continue the turn. Successful stages produce no output.

Missing jq produces a JSON warning that validation is unavailable and checks
did not run. It exits without requesting continuation: without the parser,
the hook cannot read its recursion guard. This is not successful validation.

The hook has a 120-second timeout. The runner reports timeouts; they are not
successful validation. Coverage, duplication, supply-chain checks, frontend
checks, and full release validation are separate workflows.

The JSON boolean `stop_hook_active: true` skips repeat validation to avoid
continuation loops. A false or absent flag allows checks. This corrects the
upstream guard, which skips whenever the field appears. A skipped retry does
not establish that the original failure was fixed; rerun the failed check.
Malformed payloads and invalid worktree paths report unavailable validation.

## Enable and verify

Review and trust the changed Stop definition in Codex `/hooks`; follow the
[hook setup guide](git-status-hook.md#enable) if the client retains an old
definition. Trust remains user configuration. Do not bypass it.

Run the behavioral tests with:

```bash
python3 -m unittest discover -s .codex/hooks/tests -v
```

These use temporary Git repositories and a mocked Cargo executable. They
verify orchestration, not Rust gates or delivery by the Codex client. An
actual completed turn after hook trust is required to verify delivery.
