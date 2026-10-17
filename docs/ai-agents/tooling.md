# Tooling status and prerequisites

The repository contains documentation, agent workflows, hooks, and a minimal
Cargo workspace with the Axum status application and a dependency-free xtask
crate. The Svelte/Vite frontend builds locally; release CI remains pending.
Workflow wrappers do not implement missing commands.
An informational [Codex Git-status hook](git-status-hook.md) is registered;
it requires local
prerequisites and hook trust before it can run automatically.

A [Stop validation hook](stop-check-hook.md) is also registered for changed
Rust files. It requires Bash, Git, and jq; Rust validation additionally needs
the Rust toolchain with rustfmt and Clippy. Its Cargo stages are implemented
for the full workspace. The hook reports unavailable or failing stages.

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
lockfile pins application dependencies. Available checks are:

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
library and can also run directly with `rustc --test`. The workspace checks
include application tests as well as repository tooling; they do not establish
a release or deployment pipeline.

## Application and frontend commands

See [local development](../development.md) for prerequisites, build identity,
configuration, and run instructions. These commands are implemented directly;
they are not new xtask subcommands:

| Capability | Entry point |
| --- | --- |
| Backend build/run | `cargo build --locked -p silkboard-web`, `cargo run --locked -p silkboard-web` |
| Frontend install | `npm ci --prefix frontend` |
| Frontend checks | `npm run check:all --prefix frontend` |
| Frontend build/dev | `npm run build --prefix frontend`, `npm run dev --prefix frontend` |
| Frontend advisory audit | `npm audit --prefix frontend` |

The frontend check includes types, ESLint, Prettier, component tests, and a
production build. Rust formatting, Clippy, docs, and tests cover the full
workspace. Release backend builds additionally require the identity inputs
documented in the development guide. No automated aggregate validation,
coverage, duplication, supply-chain, or browser-test gate is wired yet.
Standalone tools can be used when installed; that does not implement the
pending xtask entry points below.

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
| Frontend | Isolated Playwright suite and aggregate gate integration |
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
