# Canon provenance

## Agent progress foundation, 2026-10-18

The owner agreed milestone 2 as publish-only progress dogfooding and authorized
issue #34. This supersedes proposed task fetching/control; the roadmap records
the agreed scope and independence from outstanding deployment drills. The new
ingestion service is local Silkboard code, using rusqlite's documented
transactions/migration APIs, getrandom entropy and SHA-256 token hashing.
No upstream application code, guidance baseline, production configuration
or updater policy changes. The dependency gate checked newly resolved versions;
`cc` 1.6.0 and transitive `syn` 3.0.6 retain the 14-day cooldown. The permissive
Zlib license is added for foldhash; ARM64 CI installs SQLite's C compiler.

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
dashboard. This is an owner decision, not an upstream template sync.
The public trust configuration is installed; production admission of signed
`v0.1.1` is recorded in [updater evidence](../automatic-upgrades.md#verification-status).

The owner also selected `breki/silkboard/.github/workflows/release.yml` as the
required release suite, triggered by the release tag and checking its exact
commit. On 2026-10-17, GitHub's workflow API reported the active release
workflow ID `387356305` after PR #8 merged. This records registration,
not a successful tag-triggered release run.

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

## Production VM bootstrap, 2026-10-17

Used the completed homelab trial runbook at revision
`2677b374891e0a26004fb233081b9b528ccbef81` as the source for Debian image,
libvirt/passt, cloud-init, guest-agent and resource configuration. The new
bootstrap uses a separate production domain/disk and fixed RAM without a
balloon device. It preserves the trial and existing host services.

Reviewed MuTrack's tag updater, Phren's pre-build signature verification,
and Hoard's release guard and proposed pull updater through the
[roadmap sources](../roadmap.md#existing-patterns-checked). The new scripts
provide initial provisioning only; they do not claim to implement Hoard's
proposal or Silkboard's automatic updater. This does not advance either
application or guidance template baseline.

## Automatic updater, 2026-10-17

Adapted the ten-minute polling and atomic release-switch pattern from MuTrack
`5bdf415cbae23835c8449cf3ef3747aa8583be8f`, pre-build signature verification
from Phren `effd71bed44dd95fcbe7a4d1f6f609c2b58e05ba`, and release separation
from Hoard `79c06892e3eff1e89782b8834813ffdcd1a4b0ed`. Hoard's pull-updater
document remains a proposal, not an implemented source of deployment evidence.

Silkboard adds its own Rust state machine, exact workflow/run/job admission,
separate credential-bearing downloader, unprivileged build, root-owned staging,
and persistent interrupted-upgrade recovery. The owner selected a restricted
fine-grained token through the question UI. Its private phone-accessible
intake, protected installation, required API reads, initial-release admission,
and scheduled polling were verified on the production guest. Automatic
upgrade and recovery demonstrations remain pending; see
[the updater procedure](../automatic-upgrades.md). This does not
advance the application or guidance template baseline.

## Deployment toolkit feasibility, 2026-10-17

Issue #14 authorizes investigation of reuse inside Silkboard before possible
later extraction. The [assessment](../deployment-toolkit-feasibility.md)
inspects main `4ea173495bf4cf7c547422e3a77533e1384b0619`, including the
updater, evidence/OS boundaries, shell handoffs, provisioning and tests. It
rechecks the existing MuTrack, Phren and Hoard revisions recorded above and
links exact source files; Hoard pull deployment remains a plan. GitHub REST
run and attempt-job documentation was checked for the evidence interface.

Library/CLI profiles, state compatibility work, second-consumer adoption and
extraction criteria are proposals, not delivered capabilities or approved
milestones. No upstream code, template baseline, application behavior,
production configuration or trust policy changes in this investigation.

## Deployment toolkit implementation, 2026-10-17

The owner explicitly authorized the initial implementation after the
feasibility assessment, superseding its suggested wait before prototype work.
The existing Silkboard updater supplies the engine, Linux adapter, shell
credential/lock handoffs and default trust policy; no upstream implementation
is newly imported. The [toolkit procedure](../deployment-toolkit.md) records
new typed reports, validated policy, legacy/schema-1 state compatibility and
installation boundaries. Production code is not replaced by this change;
second-consumer adoption and repository extraction are not authorized.

## Trial cleanup, 2026-10-17

Issue #2 authorizes removal of the completed resource trial. The
[trial report](../deployment-vm-trial.md) preserves its existing measurements
and records live libvirt/storage/process/service/DNS/HTTPS checks from malina.
The owner removed the identified Tailscale device; host tailnet readback
confirmed its absence. The sibling homelab trial runbook supplies the original
resource inventory and is updated with the removal. No upstream source,
template baseline, production release or shared resource policy changes.

## CI caching, 2026-10-17

The owner authorized caching pinned CI tools and npm downloads after observing
tool installation dominate run 39044784280. GitHub's cache action and
setup-node documentation supply restore/save and npm cache semantics. The
new cache action is pinned to official v4.3.0 commit
`0057852bfaa89a56745cba8c7296529d2fc39830` (2025-09-24), without changing
existing tool or dependency versions. Exact version/workflow keys, separate
PR namespaces and pre-application cache saves are Silkboard choices. This
supersedes the earlier fresh-only CI tool installation policy; release trust
and validation gates remain required. No application/template baseline or
production installation changes.

## Documentation-only PR validation, 2026-10-17

The owner authorized issue #22 after selecting CI optimizations. Silkboard's
existing canon-check supplies link and tooling-claim verification; new Rust
classification checks Git's exact merge identity and raw file records before
selecting that limited PR scope. No upstream implementation is imported.
This changes PR validation policy only; full local validation, main/release
gates and the updater's exact required release job set remain intact.

## PR concurrency, 2026-10-17

Issue #21 authorizes cancellation of superseded PR CI. GitHub's concurrency
and reusable-workflow context documentation supply the group/caller behavior.
Silkboard uses a dedicated namespace, the caller workflow reference and PR
number, with unique run/attempt groups for non-PR events. No new dependency,
validation bypass, release admission or production policy is introduced.

## Live registry concurrency, 2026-10-17

Issue #23 authorizes bounded concurrent live requests without changing
dependency policy. The existing Silkboard registry verifier supplies all age,
source, license and integrity checks. The
[crates.io data-access policy](https://crates.io/data-access#api), checked
against its official page source, requires one API request per second and an
identifying user agent; a per-invocation pacer and repository contact URL
implement that requirement. Four-request batches, in-invocation npm
deduplication and stopping after a failed batch are Silkboard choices.
No upstream code, new dependency, verdict cache, release admission or
production configuration is introduced.

## Rust build-cache investigation, 2026-10-17

Issue #24 authorizes investigation and measurement, not adoption. A temporary
branch-only workflow used existing pinned actions and tools to measure cold,
warm and source-edit dependency-build reuse on GitHub runners. The
[research record](../ci-rust-cache-study.md) preserves run identities,
measurements, cleanup, and a proposed follow-up. Cargo's build-cache reference,
the pinned cargo-llvm-cov clean implementation and GitHub's cache reference
supply the documented artifact and isolation semantics. The research itself
adopted no upstream code, dependency, CI/release policy or production change.

## Rust dependency-build caching, 2026-10-18

After the #24 research, the owner explicitly chose to create a follow-up issue
and implement the proposed caches in a separate PR. Issue #31 supersedes the
investigation-only boundary for that implementation. The
[cache procedure](../ci-rust-build-cache.md) describes three dependency-only
archives, configuration fingerprints, exact-head PR namespaces and verified
workspace cleanup. Existing Cargo/cargo-llvm-cov cleaning and the already
pinned cache action are reused; new Rust helpers own planning and verification.
No new dependency, required release job or production configuration is added.

## Live rollback verification planning, 2026-10-18

Issue #17 requests production failure/retry evidence. The owner selected two
valid patch releases for a candidate-specific startup fault and subsequent
newer-release verification. The [drill](../live-rollback-drill.md) is a
Silkboard operator procedure matched to installed updater `4975637`, not an
installation of the newer toolkit. Read-only guest checks and its journal
support the baseline and existing v0.2.0 activation evidence. The procedure
does not itself authorize publication or disruption. After separate owner
approval, phase 1 observed real activation failures, healthy rollback, durable
blocking and explicit retry using ordinary admission. Separately approved
phase 2 admitted and activated v0.2.2 with the old block retained, then restored
and observed scheduled polling. The records identify both exact tag-triggered
suites and guest invocation/state/health evidence. Reboot acceptance remains
pending. No upstream code or deployment policy is changed.
