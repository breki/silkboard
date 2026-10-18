# Changelog

## Unreleased

- Add phone-friendly first-owner registration using expiring one-time setup
  links, atomic permanent closure and publisher ingestion locked until owner
  initialization completes.

- Publish concise Codex turn and subagent observations through native async
  hooks, with bounded first-sentence summaries, stable native identities and
  opt-in private configuration; publishing failures never control a turn.

- Add an installable standalone Rust progress publisher with private token
  files, native-session counters and bounded best-effort delivery, without
  a background service or outgoing queue.

- Add the owner mobile progress feed with login/logout, distinct agent
  sessions, paginated history, visible-page refresh and safe text display.

- Add optional single-owner password authentication with durable expiring
  browser sessions, logout, operator password reset and protected owner routes.

- Add optional SQLite-backed, project-scoped agent progress ingestion with
  persistent session identity, retry-safe acknowledgements and local publisher
  credential issuance/revocation. Existing deployments remain status-only
  until storage is explicitly configured.

## 0.2.2 - 2026-10-18

- Verification-only patch for the owner-approved newer-release upgrade drill
  in issue #17; application behavior is unchanged.

## 0.2.1 - 2026-10-18

- Cache isolated normal, coverage and ARM64 dependency builds in CI while
  retaining fresh workspace builds, tests, coverage evidence and release checks.

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
