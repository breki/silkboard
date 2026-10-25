# Changelog

## Unreleased

- Identify publisher builds with the shared repository version, source commit
  and explicit tagged/development provenance in `--version`.

- Add a Flat list / Session hierarchy switch to Events, with grouped parent
  and subagent sessions, independent collapse controls and explicit missing
  parent labels while preserving filters, pagination and event details.

## 0.9.0 - 2026-10-25

- Link explicitly associated Events to their existing Workflows, preserving
  per-event identity across sessions and leaving unknown associations unlinked.

- Rename the owner-facing Task plans feature to Workflows while preserving
  graph behavior, existing links, publishing contracts and received data.

- Name native Codex sessions after the current Silkboard task through a bounded
  local metadata helper and task-start/title-change workflow guidance.

## 0.8.0 - 2026-10-25

- Add project and agent filters to Events and Workflows, received-time bounds
  for Events, and reported-status filtering for Workflows across paginated history.

- Add opt-in local Cargo build reuse across worktrees with isolated coverage
  and ARM64 output, whole-command locking and runtime xtask workspace selection.

- Add Light, Dark and System themes with browser-local preferences and shared
  colors for reports, graphs, dialogs and account controls.

## 0.7.1 - 2026-10-25

- Show Events and Task plans directly in the compact header instead of hiding
  those primary links in the hamburger menu.

## 0.7.0 - 2026-10-25

- Clarify task-plan freshness and verify full-snapshot recovery across lost
  updates, ambiguous acknowledgements, timeouts and interrupted handoffs.

- Add owner-only task-plan graphs with directed prerequisites, step details,
  phone dialogs, received history and pinned revisions alongside the event feed.

- Fix intermittent publisher CI failures when a rejected CLI closes stdin
  before its test helper writes the input.

- Publish explicit Codex task plans with native session identity, separate
  operator opt-in and workflow guidance for revisions and writer handoff.

- Reduce full-validation work through verified Rust publication-date evidence,
  normalized dependency-cache keys, cheaper workspace test builds, fresh
  browser binary reuse and bounded parallel scheduling.

- Report elapsed time for each validation gate, including failures, and
  document a measured breakdown of representative full CI runs.

- Publish evolving task plans from the standalone CLI with synchronized local
  counters, automatic definition revisions and explicit writer handoff.

- Store bounded evolving task-plan snapshots with fenced writer handoff,
  received history and owner-only private reads, alongside existing progress.

- Preserve compatible CI Rust dependency caches across unrelated tooling
  changes while retaining cache-management and build-configuration inputs.

- Update Tokio to 1.53.3 to address the AsyncFd I/O-safety advisory
  RUSTSEC-2026-0386, and patch Undici in frontend test tooling without
  relaxing audit requirements.

## 0.6.0 - 2026-10-23

- Render multiline event Markdown safely and accept line breaks/tabs in
  bounded message inputs; hide Event ID and Sequence from details and use
  fixed local 24-hour timestamps with minute-only table times.

## 0.5.1 - 2026-10-23

- Compact the header, move owner actions into a top-right navigation menu,
  and put application status on a separate reloadable page.

## 0.5.0 - 2026-10-23

- Show individual events across all projects in one compact starting table,
  with inline details, bounded owner-only reads and automatic refresh.

- Refresh development VM GitHub CLI authentication from the vault-supplied
  token, independently of publisher credentials, with bounded private handling
  and local logout when the token is removed.

- Install the pinned Kozmotic prerequisite during development VM provisioning
  so the configured Codex Git-status hook can run on a fresh VM.

- Install Tailscale and GitHub CLI during native development VM provisioning;
  keep device registration and account authentication explicit operator steps.

- Remove private deployment addresses from tracked examples and defaults;
  require the Bombyx publisher endpoint in external operator configuration.

## 0.4.0 - 2026-10-22

- Fix new frontend-tool advisories with Vite 8.1.6 and matching Vitest 4.1.12
  packages; preserve registry integrity checks and exact cooldown exceptions.

- Fix publisher TLS certificate distrust handling with rustls-webpki 0.103.16
  and webpki-roots 1.0.10, retaining exact security-fix cooldown exceptions.

- Prepare native Bombyx development provisioning with Codex/Claude, private
  publisher secret refresh and a separate VM release signer; production trust
  installation remains a separate operator action.

- Add reusable private owner-setup handoff tooling for new servers, with
  configurable identity/application settings, retained installation and an
  automatically closing HTTPS listener.

## 0.3.0 - 2026-10-19

- Prepare production agent-progress dogfooding with opt-in private service
  storage, consistent SQLite snapshots and safe isolated restore. Align current
  architecture and agent guidance with publish-only observation.

- Add owner-only publisher credential issuance and revocation with safe metadata
  pages, one-time phone copy/download and transaction-time session checks.

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
