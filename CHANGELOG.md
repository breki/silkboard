# Changelog

## Unreleased

- Run live dependency-registry checks with bounded concurrency and explicit
  crates.io pacing, preserving dependency policy and fatal registry errors.

- Cancel superseded PR CI runs while preserving separate PRs, main and release
  runs.

- Validate documentation-only PRs with canon/diff checks; preserve full checks
  for code, uncertain changes, main and releases.

- Cache pinned CI gate tools and npm downloads while retaining full validation
  and separate PR cache keys.

- Remove the completed resource-trial VM and its registration, retaining
  measurements and shared production virtualization infrastructure.

## 0.2.0 - 2026-10-17

- Add an in-repository deployment engine, validated release policy, structured
  operator reports and policy-bound state, preserving Silkboard commands.
- Add verified release polling, isolated download/build accounts, durable
  activation recovery and failed-tag retry tooling; live verification pending.
- Add repeatable production VM bootstrap, isolated in-guest release builds,
  a sandboxed application service, and deployment verification guidance.

## 0.1.1 - 2026-10-17

- Preserve signed tag objects during GitHub release checkouts by selecting
  the event commit explicitly.

## 0.1.0 - 2026-10-17

- Add the minimal Axum and Svelte status application with build version,
  release tag, commit, and health information, plus local development checks.
- Add fail-closed validation, coverage, duplication, dependency policy and
  browser gates, plus signed-tag CI/release workflows and preparation guidance.
