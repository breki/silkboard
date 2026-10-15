# Tooling status and prerequisites

The repository contains documentation, agent workflows, and a Git-status hook.
It has no Cargo workspace, application, frontend, xtask, or release CI. The
existence of a wrapper does not change that status. A separate informational
[Codex Git-status hook](git-status-hook.md) is registered; it requires local
prerequisites and hook trust before it can run automatically.

## Available now

Git inspection, diff checks, documentation editing, review, handoff, planning,
and local reports can work with the tools actually available to the agent.
GitHub operations require working authenticated access and task authorization.
Check availability without printing credentials. Missing tools must be reported.

## Pending rustbase adaptation

The following are intended interfaces, not commands that currently work:

| Capability | Intended entry points |
| --- | --- |
| Build checks | `cargo xtask check`, `clippy`, `fmt` |
| Tests | `cargo xtask test [filter]`, `coverage`, `dupes` |
| Full validation | `cargo xtask validate --check` |
| Supply chain | `cargo xtask deny`, `audit`, `dep-age-check`, `dep-preflight` |
| Canon | `cargo xtask canon-check` |
| Records | `cargo xtask records-check`, when record tooling is adopted |
| Rust docs | `cargo xtask doc` |
| Frontend | Configured frontend checks and isolated Playwright suite |
| Backlog/changelog | `cargo xtask todo`, `changelog add` |
| Template maintenance | `sync-candidates`, `feedback-add`, backfeed helpers |

Before a workflow invokes a pending interface, inspect manifests, Cargo
aliases, command implementation/help, and required external tools. A manifest
alone does not prove a subcommand exists. If the required gate is absent,
report it as unavailable and stop the dependent operation. Do not silently
substitute weaker checks, create missing tooling, or cut a release anyway.
Planning and explicitly documented manual bookkeeping remain available.

When installing rustbase, update this status from actual commands. Wire
canon-check into validation to check supported repository path references,
relative links, and documented command availability. Keep the supported
reference syntax explicit and check facts, not prose taste. Treat descriptions
of absent or upstream paths as such rather than inventing files to satisfy
checks. Add record validation when structured IDs and their helpers exist.
Future automated checks must cover both agent wrappers and shared sources.

The Git-status hook is informational, not enforcement. Any later validation
hooks should call shared checks and accurately state which gates they run.
