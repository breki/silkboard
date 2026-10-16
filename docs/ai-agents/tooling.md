# Tooling status and prerequisites

The repository contains documentation, agent workflows, hooks, and a minimal
Cargo workspace with a dependency-free xtask crate. It has no application,
frontend, or release CI. Workflow wrappers do not implement missing commands.
An informational [Codex Git-status hook](git-status-hook.md) is registered;
it requires local
prerequisites and hook trust before it can run automatically.

A [Stop validation hook](stop-check-hook.md) is also registered for changed
Rust files. It requires Bash, Git, and jq; Rust validation additionally needs
the Rust toolchain with rustfmt and Clippy. Its Cargo stages are implemented
for the tooling workspace. The hook reports unavailable or failing stages.

## Available now

Git inspection, diff checks, documentation editing, review, handoff, planning,
and local reports can work with the tools actually available to the agent.
GitHub operations require working authenticated access and task authorization.
Check availability without printing credentials. Missing tools must be reported.

Backlog capture and listing use authenticated GitHub tooling directly; see
the [todo workflow](workflows/todo.md). GitHub issues are the backlog, with
no parallel local Markdown task queue. No xtask helper is required.

The [Cargo workspace](../../Cargo.toml) follows Bombyx's workspace/xtask
layout, with Rust edition 2024, shared lints, and 80-column rustfmt. The
lockfile has no third-party dependencies. Available checks are:

| Capability | Entry point |
| --- | --- |
| Rust formatting | `cargo fmt --all -- --check` |
| Rust linting | `cargo xtask clippy` (workspace, all targets, denied warnings) |
| Rust docs | `cargo xtask doc` (public and private passes, denied warnings) |
| Tests | `cargo xtask test` (all workspace tests) |

`cargo xtask --help` lists the implemented tasks. Tasks reject unsupported
arguments and stop on a failed Cargo stage. They use the workspace root and
locked dependency resolution. Cargo builds also work with `--offline`.

The [Stop hook's behavioral tests](stop-check-hook.md#enable-and-verify) are
registered as xtask's `stop_check` integration test. They use only the standard
library and can also run directly with `rustc --test`. These checks validate
repository tooling; they do not establish an application or release pipeline.

## Pending rustbase adaptation

The following are intended interfaces, not commands that currently work:

| Capability | Intended entry points |
| --- | --- |
| Build checks | `cargo xtask check`, `fmt` |
| Tests | Filtered `cargo xtask test [filter]`, `coverage`, `dupes` |
| Full validation | `cargo xtask validate --check` |
| Supply chain | `cargo xtask deny`, `audit`, `dep-age-check`, `dep-preflight` |
| Canon | `cargo xtask canon-check` |
| Records | `cargo xtask records-check`, when record tooling is adopted |
| Frontend | Configured frontend checks and isolated Playwright suite |
| Changelog | `cargo xtask changelog add` |
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
