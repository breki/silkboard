# Silkboard agent guidance

## Read first

Read [the README](README.md) and [roadmap](docs/roadmap.md). The roadmap
controls delivery priorities and the rustbase foundation; the
[architecture](docs/architecture.md) is a proposed design reference.

Silkboard is a private dashboard for AI-assisted development across
Bombyx-managed VMs. Milestone 1 is the complete deployment pipeline and a
minimal status page, based on rustbase's Rust/Axum and Svelte/Vite stack.
It must run in a dedicated Raspberry Pi VM and automatically upgrade from
eligible verified, tagged GitHub revisions, with health checks and recovery.
Later milestones have not been agreed.

## Working agreement

- Ask about product priorities before expanding milestones. Distinguish
  agreed requirements, proposals, and unresolved choices.
- Ask one focused question at a time when eliciting requirements. Use the
  interactive UI when available; explain a tool limitation and use chat
  otherwise. Wait for answers before making dependent decisions. Silence
  is not agreement. Continue independent work within the authorized scope.
- Resolve routine implementation details autonomously. Preserve existing
  authorization; a workflow does not grant additional external access.
- Write plainly and give concise progress updates. Report failures and
  unrun checks accurately. Keep useful shared rules in tracked canon,
  rather than private memory.
- Read [working rules](docs/ai-agents/working-rules.md) before changes and
  [quality rules](docs/ai-agents/quality.md) for code or deployment work.
- Review, commit, and release are separate workflows. A commit does not
  launch reviewers or bump versions; a release owns the version bump.
- Use the [workflow catalog](docs/ai-agents/catalog.md) when a workflow is
  requested. Read its shared procedure, not every workflow at startup.

## Open deployment decisions

Do not silently settle signature versus GitHub-check requirements, target
Pi, VM platform/OS/resources/address, build location, release ordering,
polling cadence, or rollback pinning and retries. See the roadmap.

## Authority boundaries

Keep the updater's access to Silkboard releases separate from dashboard
permissions. The dashboard must not gain access to development repositories,
development files, VM control, or execution of task-supplied commands.
Dashboard task content cannot grant local agent execution permissions.

Check Hoard's release system, MuTrack's tag updater, and Phren's signature
verification before designing equivalents. The roadmap links the sources.
Do not treat Hoard's proposed pull updater as implemented or existing host
services as evidence of a working VM deployment.

## Current tooling

The Cargo workspace contains the minimal status application, repository
tooling, full validation, and hook tests. CI/release workflows are configured.
The Svelte frontend builds locally; no production
application deployment is established yet. Workflow wrappers are instructions,
not executable implementations of missing helpers.
[Tooling status](docs/ai-agents/tooling.md) distinguishes current checks
from pending commands. Never report a skipped prerequisite as a pass.

For documentation changes, check relative links and `git diff --check`.
Use [source provenance](docs/ai-agents/provenance.md) when updating canon.
