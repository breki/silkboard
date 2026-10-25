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
Milestone 2 is agreed: publish-only agent progress dogfooding, Codex first.
GitHub remains the task tracker. Agents never fetch dashboard tasks/content
or receive execution permission from Silkboard. Milestones 3–5 cover the
owner-agreed UI, build-efficiency and task-plan work in the roadmap; scope
after milestone 5 is unset.

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
- For Codex task plans, follow [reporting guidance](docs/ai-agents/codex-plans.md)
  when the operator has explicitly enabled compatible installed components.
  Reporting failure does not block work or grant execution permission.

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

The Cargo workspace contains the status/progress application, standalone
publisher, updater, repository tooling, full validation and hook tests.
Owner setup/feed/token management and Codex reporting are locally verified;
production v0.3.0 has private state and verified owner phone registration/login.
Frosti publishes real reports from two Codex sessions; owner-visible feed/history,
live access denials and application restart persistence are verified. A tagged
upgrade with existing data remains #39; backups/recovery are deferred to #52.
Follow
the [rollout procedure](docs/dogfood-rollout.md), and keep production claims
separate from local snapshot/process tests. CI/release workflows are configured.
The minimal application runs in its dedicated production VM with private
HTTPS; automatic upgrades and live rollback/retry are verified. Updater reboot
recovery remains pending. Workflow wrappers are instructions, not executable
implementations of missing helpers.
[Tooling status](docs/ai-agents/tooling.md) distinguishes current checks
from pending commands. Never report a skipped prerequisite as a pass.

For documentation changes, check relative links and `git diff --check`.
Use [source provenance](docs/ai-agents/provenance.md) when updating canon.
