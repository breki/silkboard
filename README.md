# Silkboard

A private project dashboard for AI-assisted development, with persistent session
handoffs and task coordination across [Bombyx](https://github.com/breki/bombyx) VMs.

Silkboard helps you return to a project after days or weeks and see what was
completed, what is in progress, what is blocked, and exactly where to resume.
You can add tasks remotely; project agents fetch them and publish progress.

## Planned architecture

- A dedicated dashboard VM on a Raspberry Pi, accessible through Tailscale
  HTTPS.
- A web interface and project-scoped API backed by SQLite.
- A sync client inside each participating Bombyx development VM.
- No dashboard access to development repositories, files, or VM control.
  A separate updater can fetch Silkboard releases from GitHub.
- Local execution permissions that remain independent of dashboard task content.

## Status

The minimal Rust/Axum and Svelte/Vite status application is implemented.
Production deployment, automatic upgrades, and the sync client remain pending.
See [local development](docs/development.md) for build, run, and check commands.

Read [the architecture](docs/architecture.md) for requirements, security boundaries,
API design, and Bombyx lifecycle integration.

## Roadmap

1. Complete deployment pipeline with a minimal status page: based on
   [rustbase](https://github.com/breki/rustbase), running inside a Raspberry Pi
   VM, and automatically upgraded from verified, tagged GitHub revisions.

See [the roadmap](docs/roadmap.md) for completion criteria, existing deployment
patterns, and open decisions. The status application is available locally;
the deployment pipeline remains pending. Later milestones remain to be defined.

## Agent guidance

Start with [AGENTS.md](AGENTS.md). Codex and Claude Code share the same
[workflow canon](docs/ai-agents/catalog.md); their wrappers contain only
routing instructions. The Cargo workspace provides the application, repository
checks, and hook tests. The frontend has local build and checks; full release
gates are configured. See [tooling status](docs/ai-agents/tooling.md) for
available commands and [release preparation](docs/releases.md) for evidence
requirements.
