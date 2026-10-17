# Canon provenance

Adapted on 2026-10-15. These are guidance-source baselines, not a record that
Silkboard has imported the complete application template.

| Source | Revision | Scope |
| --- | --- | --- |
| [rustbase](https://github.com/breki/rustbase) | `6528907cea4e1e2e369fe260ccd0432b29c557b3` | Agent entry point, tool/script guidance, web patterns, workflow catalog |
| [Bombyx](https://github.com/breki/bombyx) | `38fe77286f55a0bcc6e12bd5d4500e72ae4ebb06` | Clean local checkout; working rules, quality, review separation, workflows, report template |

The [catalog](catalog.md) maps individual workflows to their source families.
The reused report template and adapted guidance retain the upstream
[MIT notice](upstream-license.txt).

This is a selective rewrite, not a promise of verbatim upstream behavior.
When syncing, inspect the upstream changes since these revisions, adapt each
relevant change, and record the scope actually reconciled. Do not advance a
whole-template watermark for a guidance-only update.

## Intentional differences

- One shared canon, with AGENTS and Claude entry points and thin wrappers.
- Both Codex and Claude supported; no assumption that their tools are identical.
- Review is separate from commit; reviewer selection follows the change.
- Behavior-focused TDD; deployment contracts can be tested without a separate
  approval question. Simple documentation work uses lightweight checks.
- Rustbase quality thresholds retained; absent automation marked pending.
- No CLI-only, Windows-only, host-specific, or speech-summary requirements.
- No copied incidents, private host addresses, stale crate names, or fake
  command availability. No permission overrides imported.
- GitHub issues are the backlog; manual changelog/feedback capture is allowed
  before helpers exist. Missing release gates block releases rather than
  being skipped.
- Full workflow set retained, including reports and template maintenance,
  with supporting assets shared and capability limitations explicit.
- Deployment eligibility and platform choices remain the owner's decisions.

Check links, wrapper routing, workflow prerequisites, and source consistency
when changing this layout. Update the shared body once rather than editing
both wrappers. Record future generic improvements through template feedback.

## Hook adaptation, 2026-10-16

Adapted Bombyx's `.claude/hooks/stop-check.sh` and its Stop registration from
`38fe77286f55a0bcc6e12bd5d4500e72ae4ebb06` for project-local Codex use. This
adds the formatting, Clippy, Rust documentation, and test stages without
advancing the guidance baseline. The adaptation parses the payload with jq,
uses its checkout path, corrects the boolean recursion guard, includes Rust
deletions, and reports missing prerequisites. Without jq it warns without
requesting repeated continuation. See the [hook guide](stop-check-hook.md).
Its behavioral regression tests use Rust's standard library and test harness,
registered with Cargo and also runnable directly with `rustc --test`.

## Cargo bootstrap, 2026-10-16

Adapted Bombyx's root manifest, Cargo alias, rustfmt configuration, and
Clippy/doc/test invocation patterns from
`38fe77286f55a0bcc6e12bd5d4500e72ae4ebb06`. The workspace currently contains
only a dependency-free xtask and the Stop hook integration tests. Shared
warning, unsafe-code, documentation, pedantic, and performance lints apply.
Rustdoc checks both public and private items with denied warnings. Bombyx's
application, dependency list, coverage exclusions, release-fast profile,
Windows helpers, and broader xtask command set were not imported. The
rustbase application and full quality gates remain pending; this does not
advance either whole-template baseline.

## Backlog policy, 2026-10-16

The owner selected GitHub issues instead of a local Markdown backlog.
The todo, backlog-report, and record procedures now use `breki/silkboard`
issues as the authoritative task queue. Local `docs/issues/` files remain
temporary implementation notes. This is an owner-directed policy change,
not an upstream template sync.

Following the owner's request for issue relationships and milestones, the
todo procedure records authorized organization using native GitHub dependency
links and agreed milestone membership, with a read-back verification step.

## Local handoff storage, 2026-10-16

The owner selected `.local/handoff.md` as the default session handoff path,
with `.local/` gitignored. Handoffs hold temporary resume context; tracked
canon and GitHub issues retain durable decisions and pending work. This is
an owner-directed storage policy change, not an upstream template sync.

## Actionable retrospectives, 2026-10-16

At the owner's request, the retrospect procedure now requires a disposition
for useful findings, applies authorized corrections, and presents concrete
proposals for changes needing a new decision. Verification and existing
publication boundaries remain in force. This is a local workflow improvement,
not an upstream template sync.

## Minimal application, 2026-10-17

Adapted rustbase's Axum router/static serving, Tokio listener and graceful
shutdown, and Svelte/Vite frontend from
`6528907cea4e1e2e369fe260ccd0432b29c557b3`. Retained the existing xtask and
hook tests rather than importing upstream's broader command set. Replaced
sample greeting/dashboard content with backend-owned status metadata,
explicit build identity validation, and component/HTTP tests. The upstream
MIT notice is retained in [the application notice](../../THIRD_PARTY_NOTICES.md).

Imported and trimmed upstream lockfiles, applying frontend advisory fixes
and cooldown-compatible pins. All resolved Cargo and npm versions were
checked against registry publication dates on 2026-10-17; none required a
14-day cooldown exception. This does not advance the guidance baseline or
import rustbase deployment, CI, coverage, duplication, or release automation.

## Release signer selection, 2026-10-17

The owner selected the existing Git SSH signing key as the trusted release-tag
signer through the interactive question UI. The roadmap records its public
fingerprint and the separation of private signing material from CI and the
dashboard. This is an owner decision, not an upstream template sync. Workflow
identity and installation of the public trust configuration remain pending.

The owner also selected `breki/silkboard/.github/workflows/release.yml` as the
required release suite, triggered by the release tag and checking its exact
commit. The registered numeric workflow ID still needs read-back evidence.

## CI and release gate adaptation, 2026-10-17

Selectively adapted rustbase's validation ordering, external tool invocations,
coverage/duplication thresholds, dependency registry date queries, cargo-deny
policy, and reusable CI/release structure from
`6528907cea4e1e2e369fe260ccd0432b29c557b3`. Rewrote the orchestration as a
small tested xtask library while retaining existing hook tests and Rustdoc
flags. The [upstream notice](../../THIRD_PARTY_NOTICES.md) also covers this
adaptation. This does not advance the guidance baseline.

Intentional differences: unavailable audits/registries/tools always fail;
all locked versions are age-checked without a bypass; Linux CI retains Unix
hook tests and cross-lints ARM64; coverage includes xtask business logic;
frontend policy includes license/source checks, coverage and an isolated
Chromium suite. The trusted SSH key and release workflow identity follow the
owner's interactive decisions above. No release, merge, or deployment is
implied by implementing these commands.
