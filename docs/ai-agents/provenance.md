# Canon provenance

## Release handoff retrospective, 2026-10-25

The owner requested a retrospective after v0.8.0 publication passed its exact
[release suite](https://github.com/breki/silkboard/actions/runs/40199401406),
but a subsequent production `/api/status` read still returned healthy v0.7.1.
v0.8.0 used the approved VM signer; v0.7.1 used the owner signer. Installed
production trust and updater logs were not accessible from this development
VM, so signer rejection remains a hypothesis, not a confirmed diagnosis.

The release procedure now surfaces signer/access evidence before publication
and checks live health/version after the suite when existing access permits.
It preserves publication authorization and requires separate authority for
trust changes or manual deployment. Historical trust/version statements are
qualified as recorded evidence. The same session also exposed late publisher
capability discovery and unrelated edits appearing during a CI wait; the
implementation and working procedures now check reporting prerequisites early
and recheck Git state before post-wait branch switches. No new production
access, publication, commit or review cycle is authorized by this retrospective.

## Local worktree build reuse, 2026-10-25

The owner selected [#66](https://github.com/breki/silkboard/issues/66) for
implementation. Its scope is local development dependency reuse, separate
normal/coverage/ARM64 output, source freshness and concurrent worktree safety.
The opt-in [local procedure](../local-build-reuse.md) cleans workspace
artifacts on path switches and serializes complete commands while retaining
dependency builds and Cargo freshness checks. It changes no CI, production
build or release-admission policy. Runtime workspace discovery corrects the
compiled-in xtask root exposed by shared output. Measurement evidence and
limits belong in that procedure rather than claims about production speed.

## Event Markdown and receipt-time format, 2026-10-23

The owner created #64 in Basic UI improvements, requested Markdown messages,
hid Event ID and Sequence from details, and selected full receipt timestamps
as `yyyy-mm-dd HH:mm:ss`. They clarified that table time omits seconds: `HH:mm`
above `MM-DD`. After the existing single-line input constraint was identified,
the owner explicitly selected multiline Markdown. Server and publisher message
validation now allow LF, CR and tabs within the existing 2,000-byte limit;
other controls, names, identities and request limits retain their rules.
Browser-local timezone, API identity and automatic Codex summaries remain intact.

[Marked renderer documentation](https://marked.js.org/using_pro) supplies the
raw-HTML/image overrides; [DOMPurify](https://github.com/cure53/DOMPurify) supplies
the tag/attribute allowlist and sanitized fragment output. The UI attaches only
those nodes, without raw HTML injection. Compact previews extract rendered
text. HTML stays literal, image labels do not load remote resources, and
sanitizer URL checks remain enabled. No syntax-highlighting or execution plugin
is added. The [exact security exception](../dependency-security-exceptions.md)
records why DOMPurify 3.4.17 is needed despite its short age; normal registry,
source, integrity and advisory gates still apply. Local fixtures establish
formatting and safety behavior; production verification remains separate.

## Autonomous release version selection, 2026-10-23

The owner instructed agents to choose future release version numbers using
their own judgment without asking for a version decision. The
[release workflow](workflows/release.md) now owns autonomous SemVer selection;
an explicit owner-supplied version or bump takes precedence. This supersedes
the earlier mandatory version-choice question in release and rollout guides.
The instruction delegates version selection only: validation, signing and
merge/publication/deployment authorization boundaries remain unchanged, and
existing authorization continues to apply.

## Unified starting event table, 2026-10-23

The owner created milestone 3 and issue #59, then selected Time, Project,
Agent (tool and host) and a short Message preview through the question UI.
They selected expansion beneath the clicked row for full message/session
identity. The [owner feed guide](../owner-feed.md) records those decisions
and the global owner-only event projection. Stable receipt/project/event
cursors preserve deterministic pagination without relying on mutable row IDs.
Compatible read indexes leave schema version 3 and existing records intact;
legacy project/session APIs and publisher/owner authority remain separate.

Isolated route/component/browser checks establish local behavior, not a live
production rollout. No dependency, credential, release or deployment policy
change is part of this issue.

## Independent development VM GitHub credential refresh, 2026-10-23

The operator supplied the Bombyx GitHub-token handoff: guest-side `GH_TOKEN`
refresh, a separately managed fine-grained token/vault mapping, independent
publisher failure handling, stdin-only login and removal through local logout.
The existing selected-key parser now serves both secrets; its quoting and
unsafe-file behavior remain covered by fixtures. No vault or external Bombyx
configuration is edited by this change.

GitHub CLI 2.103.0's official [login](https://github.com/cli/cli/blob/v2.103.0/pkg/cmd/auth/login/login.go),
[logout](https://github.com/cli/cli/blob/v2.103.0/pkg/cmd/auth/logout/logout.go)
and [config lookup](https://github.com/cli/cli/blob/v2.103.0/pkg/cmd/config/get/get.go)
implementations supply stdin token handling, no Git credential setup on the
token path, offline username lookup and the empty-store logout result. A
disposable empty HOME/config check on the actual Fusion guest confirmed that
result without touching its real credential store. The shared twenty-second
deadline and minimal child environment are exercised by isolated fake-CLI
tests. Real token login/private API verification remains an operator rollout
step after merge and secret refresh. Bombyx's local `shell_run` and
`refresh_secrets_then_hook` implementations confirm that refresh leaves the
guest checkout unchanged, so the guide requires a clean, idle fast-forward
before running the newly merged hook. No new dependency or signing authority.

## Development VM Git-status prerequisite, 2026-10-22

The owner's real Codex turn exposed a missing `~/.claude/bin/kozmotic` in the
development VM. The repository's existing Stop hook requires Kozmotic 2.3.0.
The [official release](https://github.com/breki/kozmotic/releases/tag/v2.3.0)
supplies both Linux packages; its SHA256SUMS and GitHub asset digests agree.
Provisioning pins archive and extracted-binary hashes, installs atomically
without overwriting an existing command, and leaves hook trust unchanged.

Kozmotic 2.3.0 was published 2026-10-15, seven days before selection. This is a
narrow compatibility exception to the fourteen-day cooldown: it supplies the
command already required by the checked-in hook and repairs the demonstrated
fresh-VM failure. No application dependency or agent credential changes.
Fixture tests establish install/rerun/refusal behavior; an actual subsequent
Codex turn remains necessary to verify hook presentation.

## Native Bombyx handoff and separate VM signer, 2026-10-22

The operator supplied the Bombyx vault handoff and selected the VM author
identity Silkboard VM <silkboard-vm@agents.invalid>. Public GitHub deploy-key
readback matched its key and write-enabled metadata; no vault/private key was
read. Bombyx's local tutorial/configuration and refresh implementation supplied
the native Ubuntu24.04 agent-user, clone, deploy-key and bare-env hook contracts.
The [VM guide](../bombyx-development.md) records those boundaries and the two-key
repository trust change, while production trust installation remains the
operator's task. Disposable signature fixtures and exact-set tests preserve
unlisted-key refusal and separation from candidate-controlled trust.

Node/rustup checksums come from their official release archives. Codex0.162.1
retains the repository's previously verified native hook version; Claude2.1.293
was selected using its registry publication date (2026-10-07). Official Codex
and Claude install/authentication docs supply user-owned CLI installation and
operator login steps. Bootstrap Node code uses the already pinned runtime,
without adding application dependencies or changing agent permission/trust
settings. No VM creation, live authentication, production trust installation,
privileged updater replacement or backup policy is claimed by preparation.

## Live Codex dogfooding and recovery deferral, 2026-10-19

The owner approved private SSH publisher issuance/installation and Codex
reporting enablement, completed native hook review and supplied a real feed/
history screenshot. The owner then confirmed two independent sessions.
Read-only production metadata verified real automatic observations, while live
HTTPS checks verified unauthorized/publisher-only owner reads and wrong-project/
revoked publishing return 401. An approved application restart retained all
seven baseline reports and both sessions, with schema/owner/publisher metadata
and protected updater configuration unchanged. No fixture was counted as live
reporting, and no credential/message contents are included in these records.

The owner explicitly deferred independent backups, isolated restore and
application recovery acceptance from #39 to #52. The
[rollout](../dogfood-rollout.md#evidence-ledger) preserves the outstanding
compatible tagged upgrade and post-upgrade phone acceptance. No new dependency,
schema, backup policy, signing trust or updater installation is introduced.

## Reusable owner handoff, 2026-10-19

After the owner confirmed production registration and phone login, they asked
to preserve the handoff for future servers. The [bootstrap helper](../../deploy/owner-handoff/README.md)
retains the verified Node transport with explicit operator configuration,
dynamic self-address checks and an installed-but-disabled service. The live
child-switch failure was reproduced in a disposable database under the same
sandbox; retaining CAP_SETUID across child execution fixed it, and the
application child had zero effective/permitted/ambient capabilities. No new
dependency, application release, publisher credential, backup policy or live
service change is introduced by this repository generalization. Local fixtures
and existing live evidence remain distinct; production first-owner setup is
closed and the original temporary deployment was removed.

## Private owner phone handoff preparation, 2026-10-19

The owner selected preparation of a temporary HTTPS channel restricted to
their Tailscale identity for #39's one-time setup link. Read-only guest checks
confirmed Tailscale 1.104.1 and the existing port 443 proxy. Tailscale's
installed-version proxy source and Serve lifecycle documentation supply the
identity-header sanitization, Unix-socket forwarding and foreground-session
contracts linked from the [setup guide](../owner-setup.md). The local
operational helper reuses the existing setup command and installed Node runtime;
no upstream code, dependency, application release, live credential or production
configuration is introduced by preparation. Isolated browser fixtures cannot
establish real phone identity or production registration. Installation remains
a separate concrete rollout decision.

## Initial dogfood rollout preparation, 2026-10-18

Issue #39 requires aligning the original architecture proposal with the agreed
milestone-2 publish-only model. README, architecture and agent guidance now
describe implemented foundations separately from production v0.2.2. The
[rollout](../dogfood-rollout.md) and [data guide](../application-data.md) record
opt-in persistent state, review/publication/installation boundaries and pending
live evidence. SQLite VACUUM INTO and systemd StateDirectory documentation
supply the snapshot and sandbox interfaces. Restore revokes copied publisher
tokens, browser sessions and pending links rather than reviving stale authority.
No new schema, dependency, backup schedule/destination, production credential
or privileged updater behavior is adopted by this preparation.

## Owner publisher credential management, 2026-10-18

Issue #44 authorizes phone issuance, safe metadata and revocation under the
existing owner login/CSRF boundary. The [manager guide](../publisher-tokens.md)
records shared publisher transaction semantics, reauthentication at mutation
commit, one-time disclosure and explicit private saving. Existing setup-page
cache/referrer/frame/content protections now cover the configured owner
application. MDN clipboard/Blob/pagehide APIs supply the browser interfaces.
No new dependency, schema, production credential, updater authority or release
policy is adopted. Production installation and live acceptance remain #39.

## One-time first-owner setup, 2026-10-18

Issue #45 and the owner's prior one-time-link decision authorize phone owner
registration before publisher access. Existing Argon2, SQLite transactions,
origin checks and hashing limits supply the implementation foundation. MDN
fragment/history APIs and OWASP high-entropy single-use token guidance inform
the [setup guide](../owner-setup.md). Schema 3 retains existing owners as closed;
the private preparation pipe keeps only verification hashes on the server.
No new dependencies, upstream code, production configuration, user credentials,
updater authority or release policy are adopted. Production rollout remains #39.

## Codex progress hooks, 2026-10-18

Issue #37 authorizes automatic Codex observations using the installed publisher.
The owner selected a bounded first sentence of final replies. OpenAI's official
hook/configuration/streaming documentation, installed Codex 0.162.1 schemas and
an isolated native runtime supplied the event and async contracts documented in
the [integration guide](../codex-publishing.md). A controlled local model provider
drove real root/child/resume/concurrent callbacks into a disposable authenticated
Silkboard backend. No upstream code, new dependency, user trust mutation,
production credentials or deployment changes are imported. Existing validation
handlers and execution authority remain intact; production dogfooding is #39.

## Standalone publishing CLI, 2026-10-18

The owner narrowed #36 to an installed Rust executable with bounded
best-effort publishing, no daemon and no outgoing queue. Codex is first;
Claude remains separate #43. The [client guide](../publishing-cli.md) records
the implemented wire contract, private token/configuration files and minimal
sequence counters. Server ingestion and Bombyx's installed-binary model supply
existing local contracts; no Bombyx implementation is imported.
[Ureq 3.4.2](https://docs.rs/ureq/3.4.2/ureq/) supplies synchronous HTTP/TLS.
New versions were checked against registry dates/licenses before compilation.
Ureq-proto 0.6.4 and zeroize 1.9.0 retain cooldown-eligible versions. The first
full audit rejected Rustls 0.23.45 with four advisories; exact security fix
0.23.46 uses the documented [narrow exception](../dependency-security-exceptions.md).
ISC and CDLA-Permissive-2.0 are added for
the TLS implementation/trust anchors, with attribution texts embedded in
`--licenses`. Existing dependency versions remain intact. Schema, production
installation, credential setup and release trust policy are unchanged.

## Owner progress feed, 2026-10-18

Issue #38 authorizes the mobile owner feed. The existing ingestion identity,
SQLite store and protected owner router supply its boundaries. New bounded
queries and Svelte components implement the [feed contract](../owner-feed.md)
without new dependencies, schema changes or additional agent authority.
The existing Playwright suite now uses a disposable OpenSSL HTTPS proxy and
the real backend to verify secure cookies and phone behavior. Production
configuration, credentials and rollout remain #39 work; broad architecture
alignment remains separate from this implementation.

## Owner authentication, 2026-10-18

Issue #35 authorizes the single-owner authentication foundation for milestone
2. The [authentication guide](../owner-login.md) records actual APIs and
configuration, using RustCrypto Argon2id, random opaque hashed sessions and
MDN's host-cookie/origin/CSRF guidance. systemd's ask-password interface
provides the documented private operator pipe. The six new resolved Cargo
versions were checked for cooldown and permissive licenses before compilation.
Schema 2 retains existing ingestion data. No upstream application code,
template baseline, production configuration, updater trust or execution
authority changes. Frontend and production verification remain separate.

## Agent progress foundation, 2026-10-18

The owner agreed milestone 2 as publish-only progress dogfooding and authorized
issue #34. This supersedes proposed task fetching/control; the roadmap records
the agreed scope and independence from outstanding deployment drills. The new
ingestion service is local Silkboard code, using rusqlite's documented
transactions/migration APIs, getrandom entropy and SHA-256 token hashing.
No upstream application code, guidance baseline, production configuration
or updater policy changes. The dependency gate checked newly resolved versions;
`cc` 1.6.0 and transitive `syn` 3.0.6 retain the 14-day cooldown. The permissive
Zlib license is added for foldhash; ARM64 CI installs SQLite's C compiler and
target libc development headers explicitly. PR CI exposed the missing headers
when recommended packages were disabled; the explicit package list fixes it.

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

## Validation-blocking security repairs, 2026-10-22

The complete VM-preparation validator passed behavior/coverage gates but failed
its mandatory live audit on new RustSec TLS and npm Vite/Vitest advisories.
The [exact security exception record](../dependency-security-exceptions.md)
links the primary advisories and observed registry dates. Two compatible Rust
transitive patches and the minimum Vite/Vitest fixes (including matching Vitest
components) preserve normal registry/license/integrity checks and fail-first
exception regressions. No generic cooldown bypass or ignored advisory is added.
These scoped repairs are required for CI/release eligibility; they do not
install production code or expand publisher/VM authority.

## Private deployment configuration — 2026-10-22

Owner requested removal of the private Silkboard Tailscale address from the
repository and defaults. Tracked deployment examples use reserved domains;
Bombyx requires an operator-supplied HTTPS origin and the application service
template leaves owner origin configuration to an external drop-in. Live
production and workstation configuration remain operator-owned. This change
does not rewrite historical commits or alter running production configuration.

## Development VM connectivity tools — 2026-10-22

The owner approved a separate Tailscale connection for the Fusion development
VM and required its installation through provisioning, alongside GitHub CLI.
Live guest inspection found private endpoint DNS unresolved before Tailscale
installation. The provisioner follows Tailscale's stable Ubuntu 24.04 package
repository instructions and GitHub's own signed package repository for its CLI;
the maintainers report deprecated API failures in Ubuntu's older package.
Installation starts the daemon but performs no account/device login, host
firewall change, route advertisement or workstation credential migration.

## Compact header and status navigation

On 2026-10-23, the owner requested #61 in Basic UI improvements: reduce the
header, remove visible Agent progress, move Sign out and Manage publisher
credentials to a top-right hamburger menu, and move Application status to its
own page reached from that menu. Implementation uses normal `/status` and `/`
links with an explicit status shell route, retaining public status and
owner-only administration. Component, HTTP and browser tests establish local
behavior; production rollout remains separate.

## Tokio I/O-safety repair — 2026-10-24

The owner authorized a separate Tokio security fix to unblock #70 and its
merge. RustSec RUSTSEC-2026-0386 lists Tokio 1.53.3 as patched; crates.io
reports publication at `2026-10-13T20:51:46.498182Z`. The exact
[security exception](../dependency-security-exceptions.md) records this
short-age adoption. The regression test failed before the exception and
passes with only that package/version/publication second admitted. Advisory
checks remain enabled; this record does not establish production deployment.

The subsequent npm audit rejected existing jsdom/Undici 7.30.0 for three
upstream advisories. The same prerequisite repair updates only the existing
override to 7.30.1, published `2026-10-15T14:18:31.283Z`; the exact
[exception record](../dependency-security-exceptions.md) links the advisories
and preserves registry/integrity checks. This is test/build tooling, with no
production installation or release implied.

## Evolving task-progress graphs — 2026-10-24

The owner requested directed task-progress graphs showing required steps,
completed work and current activity, selected an agent's evolving task-specific
plan, requested the standalone interactive mockup, and authorized milestone 5
and issues #73–#81. The prototype's task completion states are fictional.
While starting #73, the owner selected the same graph across sessions through
an explicit handoff, multiple active steps and reopening the same step while
retaining earlier completions in history.

The [reporting contract](../task-plan-contract.md) translates those choices into
a bounded full-snapshot design, separate ordering/plan revision/reporting
generation, immutable received history and private owner reads. Numeric limits
and wire rules are technical design choices for review, not additional
owner-supplied requirements. Current progress payloads, schema 3, installed
hooks and production remain unchanged. Implementation and live dogfooding
follow the linked milestone issues; the contract grants no merge, release or
new dashboard execution authority.

## Task-plan navigation — 2026-10-24

While preparing #74, the owner selected a separate Task plans page linked
from the hamburger menu, preserving the existing Events starting view. The
[presentation design](../task-plan-design.md) records the view and interaction
rules; the [tracked mockup](../mockups/task-plan.html) preserves the earlier
offline prototype for implementers. Its one-active-step fixture is illustrative;
the normative design includes the subsequently agreed multiple-active and
reopening/session-continuity behavior. No deployed route or API is implied.

During [the contract CI run](https://github.com/breki/silkboard/actions/runs/40114170874),
the jobs API returned empty step metadata even when the completed log showed
full validation and ARM64 lint. A premature stall inference caused one retry.
The [working rule](working-rules.md) now distinguishes missing telemetry from
evidence of stopped execution; no gate or merge requirement is relaxed.

## Task-plan persistence and private APIs — 2026-10-24

Issue #75 implements the previously agreed #73 wire contract using a shared
`silkboard-plan` model and additive SQLite component version 1, retaining base
schema 3. The separate component version is an implementation choice to keep
retained v0.6.0 progress/owner paths compatible; it does not authorize future
schema changes or a production rollout. Normalized snapshots are also capped
at 256 KiB so direct library writes cannot bypass the transport bound. No new
registry dependency versions are adopted.

Local route, restart and backup/restore tests check private access, ordering,
received history and authority invalidation. A separate loopback process drill
used actual signed v0.6.0 source at `07026bbd0d6a1feff9d04a4e47856d1611afa00d`
against fixture data containing plan tables: ordinary progress/feed and legacy
snapshot/restore worked, new-code reads retained plans, and restored credentials
were denied. The old binary is compatibility-test evidence, not a clean current
security-audit result. No live production migration/recovery was performed.
See the [API guide](../task-plan-api.md) for bounds and remaining integrations.

## Installed task-plan publisher — 2026-10-24

Issue #76 implements explicit plan start/publish/local handoff commands using the
shared #75 protocol. Automatic sequence/revision allocation and a separate
local handoff command are implementation choices for the agreed single-writer,
evolving-plan contract. A separate start operation refuses existing local
state; ordinary publishing refuses missing state rather than guessing counters.
Current plan content and high-water marks are private
working state, not an outbound queue. Endpoint/project configuration and
credentials remain operator-controlled; no dashboard reads are introduced.

Existing verified transport and private-file operations are shared with
ordinary progress commands. No registry versions or third-party dependency
closure are changed. Tests cover real ingestion scope/conflicts/revocation,
compiled command restarts without runtime helpers, failed sends, concurrent
allocation, corruption, return handoff and stopped-writer private transfer.
These are local evidence, not installation, native plan capture or production
phone dogfooding. See [plan publisher](../task-plan-publisher.md).

## Full CI timing analysis — 2026-10-24

Issue #86 examines six completed successful CI runs from 2026-10-24 using
GitHub job/step metadata, timestamped logs and the source at their recorded
head revisions. The [report](../ci-time-analysis.md) and extracted CSV data
retain run links, exact SHAs, cache conditions and measurement limits. All
registry-version sets are unchanged across the sample; workspace/manifest
changes, cache namespaces and runner variation remain confounders. Raw logs
stay outside tracked artifacts. No optional benchmarking reruns were launched.

One older log delayed two gate markers, so it records their combined interval
instead of claiming zero-time VM tests. The validator now reports monotonic
per-gate elapsed milliseconds on success/failure. This is observability only,
not a change in gate order, dependency prerequisites, thresholds, cache trust
or release eligibility. Ranked optimizations are proposals requiring their
own implementation decisions. Local instrumentation tests/full validation
and independent reviews are separate from the pre-instrumentation CI sample.

## Full validation optimizations, 2026-10-24

After the #86 timing analysis and PR #87 ranked proposals, the owner explicitly
asked to implement them. The [optimization report](../ci-validation-optimizations.md)
records all five changes, matched local measurements and GitHub evidence limits.
The [Cargo profiles reference](https://doc.rust-lang.org/cargo/reference/profiles.html)
defines workspace/package overrides and test-profile behavior. The
[TOML parser documentation](https://docs.rs/toml/0.8.23/toml/) supplies typed
manifest/lock parsing; it is repository tooling, not a new publisher dependency.

Before compilation, registry timestamps, checksums, licenses and advisory gates
were checked for newly resolved toml 0.8.23, toml_edit 0.22.27,
toml_datetime 0.6.11, serde_spanned 0.6.9, indexmap 2.14.2,
winnow 0.7.15 and equivalent 1.0.2. Equivalent remains pinned in the lockfile
to 1.0.2 because the newer release had not completed the fourteen-day cooldown.
No security exception or validation threshold was added or relaxed.

## Explicit Codex evolving plans, 2026-10-24

After #73–#76 merged, the owner resumed milestone 5 and work on #77. The
[Codex workflow](codex-plans.md) maintains the full task-specific graph through
explicit installed-publisher calls. OpenAI's [app-server documentation](https://learn.chatgpt.com/docs/app-server)
and installed Codex 0.163.0 generated `TurnPlanUpdatedNotification` schema
supply only text/status entries; they do not carry stable step IDs,
prerequisites or blocked reasons. The [hook documentation](https://learn.chatgpt.com/docs/hooks)
permits observing local plan-tool calls but cannot supply those missing fields.
No transcript reader or new listener/hook is introduced to reconstruct them.

A controlled-local-provider probe ran the actual installed Codex app server
with an owned temporary home and a self-authored shell command. It confirmed
`CODEX_THREAD_ID` equals the native thread ID. This is verified version-specific
behavior, not a documented permanent environment API. No real model account,
user hook trust or production configuration changed. The adapter requires that
identity and explicit `codex.plan_reporting: true`; existing summary settings
remain disabled for plans. It derives host/name from operator configuration,
checks the actual working directory and never accepts reporter or configuration
overrides in plan input. Existing synchronized plan state and bounded delivery
remain authoritative. Root writers hand off explicitly; child/Stop observations
never complete the task. Local API/CLI fixtures verify states, revisions,
reopening and replacement writers. Production installation/phone acceptance
remain separate milestone work.

## Owner task-plan graph — 2026-10-25

The owner resumed milestone 5 after #77. Issue #78 implements the accepted
[presentation design](../task-plan-design.md), including the owner's earlier
choices of a separate Task plans page, evolving definitions, multiple active
steps and retained received history. Native HTML buttons over bounded SVG
prerequisite arrows avoid an additional graph package. This is a routine
implementation choice, explained when the owner asked about dependencies.
The existing safe Markdown renderer and authenticated same-origin owner APIs
remain authoritative. The UI adds no reporting, execution or VM privileges.
Local component tests and disposable real-API browser fixtures cover snapshots,
reopening, revision selection, phone focus and the contract's maximum graph.
These fixtures are not production reports, installation or phone acceptance;
combined milestone verification and authorized rollout remain separate work.

## Task-plan reconciliation evidence — 2026-10-25

After authorizing merge #91, the owner asked to continue milestone 5. Issue
#79 verifies the existing full-snapshot recovery contract through the installed
CLI and actual authenticated Axum routes, with test-only faults before writes,
after accepted writes and across the network deadline. Exact sequence/history
checks cover lost revisions, duplicate/delayed/conflicting reports, removed
steps and interrupted writer handoff. Component and real-API browser checks
verify aging, retained states/selection and read-failure warnings. The owner
copy now explicitly distinguishes reported states from connection status.
[Recovery evidence](../task-plan-recovery.md) records the scenarios and limits;
it does not claim #80 combined acceptance or #81 production installation.

## Combined task-plan acceptance — 2026-10-25

After authorizing merge #92, the owner asked to continue milestone 5. Issue
#80 adds the installed publisher/owner graph/process restart seam and collects
[acceptance evidence](../task-plan-acceptance.md). The existing native Codex
0.163.0 probe was repeated with its actual shell invoking the compatible
installed publisher against a disposable local API. Official
[app-server](https://learn.chatgpt.com/docs/app-server) and
[provider configuration](https://learn.chatgpt.com/docs/config-file/config-advanced)
references describe the transport/setup; the native environment match is a
measured version-specific observation. Controlled provider responses are
separate from actual development-session plan reports to the same local server.
A fresh isolated drill began with data created by the retained signed v0.6.0
binary, then checked additive plan initialization and legacy-compatible recovery.
No production data, model account, user transcripts or existing user hook/reporting
configuration was used or changed. Temporary configuration opted in only for
the disposable fixture. Release and phone dogfooding remain #81.

## Production task-plan rollout — 2026-10-25

After merging #93, the owner approved a private recovery copy, publication of
the agent-selected `v0.7.0`, compatible publisher installation and explicit
Codex plan opt-in on the existing development machine. The
[live rollout record](../task-plan-rollout.md) binds signed release admission,
the exact successful release run, installed revisions, isolated snapshot
verification and production data/authority preservation to that authorization.
The normal installed updater poll was requested manually; this is not claimed
as scheduled discovery or as replacement of the privileged updater.

Actual native task reports and a controlled rejected update establish the
reported history and full-snapshot repair. The owner explicitly confirmed
received history and receipt time/age on their phone, then explicitly confirmed
all three final checks: 7 of 7 steps done, step details opening/closing, and
received Blocked-to-Active-to-Done history. Those owner observations establish
live phone acceptance; they are separate from fixture identities/states and
local tests. Reported completion grants no execution or merge authority.

## Rollout retrospective clarifications — 2026-10-25

The owner requested a retrospective after #81's evidence PR merged. A targeted
browser run in a fresh worktree failed before testing because built frontend
assets were absent despite shared dependencies; the test workflow now points
to that prerequisite explicitly. During phone acceptance, the owner asked
where history and receipt age were shown. Working rules now clarify using
visible labels/locations and recording each answer's actual scope. These are
narrow clarifications of existing testing and communication requirements.
Shared Cargo output also reused an xtask executable built for another worktree;
rebuilding it restored the intended validation root. Working rules now call
out checking that root before relying on shared helpers. These clarifications
introduce no release, external-write or execution authority.

## Direct primary navigation — 2026-10-25

The owner requested #96 in the basic UI milestone and authorized its
implementation: Events and Task plans should be direct top-row links rather
than hamburger entries. This supersedes only the entry placement in #61/#74.
The starting Events view, existing paths, owner authentication and the other
hamburger actions remain unchanged. Setup retains its navigation-free header.
Local component and browser checks verify direct links, menu boundaries,
keyboard operation and compact phone/desktop layout; they do not establish a
new production release or owner phone acceptance of this navigation change.

## Browser appearance preferences — 2026-10-25

The owner requested #99 in the UI milestone and authorized implementation of
Light, Dark and System themes, with System as the default. The selector uses
the existing menu and remains available on setup without adding navigation.
A browser-local preference is applied before application mount; CSS used color
schemes follow OS changes in System mode. Shared semantic colors cover both
base pages and task graphs, preserving text and symbols for graph states.
Storage failure does not block current-page switching or authentication.
[Appearance guidance](../appearance.md) describes the behavior and its local
preference scope. The #98 terminology change and any release remain separate.
