# Silkboard

A private project dashboard for AI-assisted development, with persistent session
handoffs and task coordination across [Bombyx](https://github.com/breki/bombyx) VMs.

Silkboard helps you return to a project after days or weeks and see what was
completed, what is in progress, what is blocked, and exactly where to resume.
You can add tasks remotely; project agents fetch them and publish progress.

## Planned architecture

- A dedicated dashboard VM on a Raspberry Pi, accessible through Tailscale HTTPS.
- A web interface and project-scoped API backed by SQLite.
- A sync client inside each participating Bombyx development VM.
- No dashboard access to GitHub repositories, development files, or VM control.
- Local execution permissions that remain independent of dashboard task content.

## Status

Architecture proposal. No application or sync client has been implemented yet.

Read [the architecture](docs/architecture.md) for requirements, security boundaries,
API design, Bombyx lifecycle integration, and implementation milestones.

## First milestone

Implement owner authentication, a project overview, status publishing, and a
local handoff format before adding task polling and claims.
