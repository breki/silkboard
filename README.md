# Silkboard

A private dashboard for short progress reports from AI-assisted development.
Silkboard's own Codex agents are the first intended production publishers.
[GitHub](https://github.com/breki/silkboard/issues) remains the task tracker;
agents never fetch tasks, dashboard content or execution instructions from
Silkboard. Bombyx-managed development VMs can use the same installed publisher.

## Architecture

- Rust/Axum and Svelte/Vite, based on [rustbase](https://github.com/breki/rustbase).
- A dedicated Raspberry Pi VM with private Tailscale HTTPS and SQLite storage.
- A standalone Rust publishing executable; target machines need no Rust or
  Cargo, daemon or durable outgoing queue. Delivery is bounded and best-effort.
- Separate owner login for a mobile project/session feed, and project-scoped
  publish-only tokens. First-owner setup and token management work through
  private HTTPS pages.
- No dashboard access to development repositories, files, VM control or agent
  execution. A separate privileged updater verifies Silkboard releases.

See [architecture](docs/architecture.md) for current components and authority,
and [the roadmap](docs/roadmap.md) for agreed scope and remaining evidence.

## Status

Production runs `v0.3.0` with private HTTPS, persistent private storage and
verified first-owner registration/login from the owner's phone.
Automatic upgrades and live rollback/retry are verified; updater reboot
recovery and live pending-check reconsideration remain separate work.
[Production evidence](docs/production-vm.md) records the original installation.

The progress API, owner feed/setup, token manager, standalone publisher and
Codex integration are implemented and locally verified. The
[dogfood rollout](docs/dogfood-rollout.md) prepares persistent private storage,
backup/restore and live acceptance. Publisher installation and real automatic
Codex reports, phone feed acceptance with real messages, and live data recovery
remain [#39](https://github.com/breki/silkboard/issues/39). Claude integration is
separate [#43](https://github.com/breki/silkboard/issues/43).

## Development and delivery

See [local development](docs/development.md) for build/run commands,
[publishing CLI](docs/publishing-cli.md) for private client configuration,
and [Codex reporting](docs/codex-publishing.md) for native hook integration.
The [application data guide](docs/application-data.md) covers private snapshots
and recovery. A repository merge does not install service configuration or
credentials on production.

Start agent work with [AGENTS.md](AGENTS.md). Codex and Claude Code share the
[workflow canon](docs/ai-agents/catalog.md). The Cargo workspace contains the
application, publisher, updater and repository checks. See
[tooling status](docs/ai-agents/tooling.md) and [release preparation](docs/releases.md).
