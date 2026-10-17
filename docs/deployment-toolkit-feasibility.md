# Reusable deployment and upgrade toolkit: feasibility

Assessment for [issue #14](https://github.com/breki/silkboard/issues/14),
2026-10-17. This assessment records the original design proposal and source
baseline. The owner subsequently authorized the first
[in-repository implementation](deployment-toolkit.md), before the remaining
live reference tests. No toolkit has been extracted, published or substituted
for the installed production updater. Later adoption/stabilization remains
subject to the evidence and owner decisions described here.

## Finding and recommendation

Reuse is feasible for the release-selection and recovery engine, and for a
narrow Linux/systemd deployment toolkit built around it. The existing Rust
crate already separates a state machine from its execution backend. It is
not yet a reusable deployment product: admission policy, account names,
filesystem layout, build recipe, artifact layout and health responses are
specific to Silkboard.

Recommend a hybrid: a Rust library for the engine and evidence validation,
plus an operator-installed CLI with reviewed platform/application adapters.
Develop that interface inside this repository first, as requested. Keep
Silkboard's strict admission and recovery policy as the reference profile;
do not make security optional merely to accommodate different applications.
Provisioning should supply the environment the toolkit requires, using
application-specific scripts rather than a new general VM manager.

The original sequencing recommendation was to start implementation after
issue #6 demonstrates
live upgrade, rollback, retry and reboot recovery. First characterize that
behavior and the current state format, then introduce one operator-owned
profile without changing Silkboard behavior. A second, separately authorized
consumer should test whether the interface is useful before extraction.
The owner has now authorized the initial local implementation. Live proof
remains required for operational claims and stabilization; the original
sequencing recommendation is not a GitHub blocking relationship.

## Evidence and current limits

Source baseline: Silkboard main
`4ea173495bf4cf7c547422e3a77533e1384b0619`. Relevant inspected components:

| Component | Observed responsibility and reuse limit |
| --- | --- |
| [Updater core](../crates/silkboard-updater/src/lib.rs) | Canonical stable versions, release identity, pending transaction, tag blocklist, `Backend`, `poll` and `retry`. No direct OS calls in the engine, but output is printed and errors are strings. |
| [GitHub evidence](../crates/silkboard-updater/src/evidence.rs) | Checks fixed repository/workflow IDs, workflow paths, exact tag/commit, current attempt and exactly three successful jobs. These constants are policy, not universal GitHub requirements. |
| [Linux backend](../crates/silkboard-updater/src/linux.rs) | Downloads, signature/evidence admission, durable state, build handoff, staging, symlink/restart and exact-identity health. Production paths, users, repository and endpoint schema are fixed. |
| [CLI entry point](../crates/silkboard-updater/src/main.rs) and [wrapper](../deploy/vm/update.sh) | Fixed commands and paths, clean subprocess environment, lock and transient credential handling. The library does not itself acquire the wrapper's lock. |
| [Build](../deploy/vm/guest-build.sh) and [stage](../deploy/vm/guest-stage.sh) | Unprivileged Rust/Svelte build, independently checked tag, regular-file snapshot, normalized ownership/modes and Silkboard artifact/identity layout. |
| [Installer](../deploy/vm/install-updater.sh), [service](../deploy/vm/silkboard-update.service), [timer](../deploy/vm/silkboard-update.timer) | Operator-owned installation, systemd permissions and ten-minute polling. Accounts, hostname and ARM64 assumptions are specific to the current guest. |
| [VM bootstrap](../deploy/vm/README.md) | malina/libvirt/passt, Debian 13 ARM64, fixed resources, toolchains and Tailscale. A deployment recipe, not an interchangeable provisioning API. |

[Production evidence](automatic-upgrades.md#verification-status) establishes
installed updater code, known-release admission, scheduled polling and
credential separation. It does not establish a complete newer-release upgrade
or failed-upgrade/reboot cycle. Existing
[recovery tests](../crates/silkboard-updater/tests/recovery.rs),
[GitHub evidence tests](../crates/silkboard-updater/tests/evidence.rs),
[real-signature tests](../crates/silkboard-updater/tests/signature.rs), and
[Linux boundary tests](../crates/silkboard-updater/src/linux/tests.rs) provide
useful contracts; they are not proof that another application's adapter is
safe or that a general toolkit works.

The existing public `Release::new` validates identity syntax, not eligibility.
`Release::from_tag` parses tag structure, while signature and GitHub checks
remain separate. A future public API must not describe those values as proof
of trusted admission merely because they have a Rust type.

## Which parts belong together

| Boundary | Candidate reusable part | Application/operator responsibility |
| --- | --- | --- |
| Provisioning | Prerequisite/account/path checks and installation contract | Choose host, VM/OS/resources, network access and toolchains; create accounts and initial healthy release. Keep VM control outside the running updater. |
| Admission | Exact identity binding, SSH-tag verification and a strict GitHub evidence evaluator | Install signer trust and repository/workflow/job policy independently of candidate source; approve policy changes. |
| Selection | Numeric stable-tag ordering, reconsideration of pending checks and failed-tag blocking | Initially retain the existing stable-tag/no-downgrade policy. Other channels require a separately designed policy. |
| Build | Credential-free source handoff and supervised unprivileged execution | Supply a reviewed build adapter and pinned tools; recipe may execute candidate build code only as the builder. |
| Staging | Snapshot and validate expected regular artifacts into protected release storage | Define permitted artifacts, executable files, identity format and runtime layout. Reject unexpected file types/paths rather than trusting a candidate manifest. |
| Activation/health | Durable transaction, atomic current-link replacement, restart and bounded health orchestration | Select one service and a health adapter that proves the running version/tag/commit, not just an HTTP 200. |
| Recovery/retry | Recover pending work first, retain the previous release, persist blocks and explicit retry | Preserve prior artifacts and compatible application data; define operator intervention when recovery cannot succeed. |
| Scheduling/logs | Serialized poll entry point and structured outcomes | Install/enable the systemd timer, set host resource limits and monitor logs, disk and credential expiry. |

A shared engine cannot make database migrations reversible. Silkboard's
current status app does not exercise that problem. Mutable application data,
incompatible schema changes, multi-service coordination and zero-downtime
requirements need separate design before promising rollback for such users.
Automatic pruning, automatic downgrade/pinning and privileged self-update
are also absent today; reuse must not imply they already exist.

## Library, CLI, or both

| Shape | Benefit | Cost or limit |
| --- | --- | --- |
| Rust library only | Direct state-machine and evidence tests; typed identities/errors; no shell reimplementation | Every consumer must write and maintain a trusted runner, lock, credential transport, installer and recovery-compatible platform backend. A public trait alone does not enforce privilege separation. |
| CLI toolkit only | Existing timer/bootstrap scripts can invoke a stable process interface | A configuration surface can accidentally become privileged command execution. Structured output, schema validation and compatibility still need an engine beneath them. |
| Library plus CLI/adapters | Test the engine once; ship a controlled Linux runner; keep build/health differences explicit | More packaging and compatibility work. Keep one internal implementation initially rather than creating several crates before a second use case exists. |

Recommend the third shape, initially limited to one application per dedicated
Linux/systemd guest, GitHub, SSH-signed stable tags, local unprivileged builds
and retained releases. Windows, alternate CI providers, artifact-download
builds, multi-application hosting and orchestration are future possibilities,
not promises or accepted scope. Existing applications need not adopt this
policy to remain useful sources of design comparisons.

## Concrete interface sketches

The following are original design sketches, not the implemented schema. The
[current interface](deployment-toolkit.md) uses validated JSON and registers
only Silkboard. These broader TOML/profile sketches remain illustrative,
not operator instructions or a public SDK commitment.

An operator-owned profile could bind identity, evidence and installed adapters:

```toml
application = "silkboard"
repository = "breki/silkboard"
repository_id = 1417054470
signers_file = "/etc/silkboard/release-signers"
release_workflow_id = 387356305
release_workflow_path = ".github/workflows/release.yml"
evidence_profile = "silkboard-release-suite-v1"
platform_profile = "linux-systemd-v1"
application_adapter = "silkboard-v1"
```

The evidence profile would bind the reusable CI path and exact required jobs,
including tag ref, commit, run and attempt validation. The application adapter
would bind service/accounts, layout, build and health behavior. Initially,
keep these reviewed mappings in installed code; do not make a filename in
candidate source select a root helper. The profile file must be root-owned,
validated, and outside builder/runtime write access. It contains references
to protected credential files, never token values. Unknown profiles, unknown
fields or changed repository identity stop admission.

The CLI could expose operations analogous to the current fixed commands:

| Proposed operation | Contract |
| --- | --- |
| `deployctl inspect PROFILE TAG` | Verify against installed policy without building or activating; distinguish pending/rejected evidence from API failure. |
| `deployctl poll PROFILE` | Acquire the profile's lock, recover first, then select/build/recheck/activate/verify at most one release. |
| `deployctl status PROFILE` | Report schema, profile identity, current release, pending transaction and blocked tags without needing network credentials. |
| `deployctl retry PROFILE TAG` | Clear only a valid named block after pending recovery completes; no forced deployment or verification bypass. |

`PROFILE` should identify a preinstalled allowlisted profile, not an arbitrary
path supplied through a dashboard request. No dashboard execution endpoint
is proposed. Structured results should distinguish unchanged, waiting,
activated, recovered, rejected and boundary failure; command exit codes and
JSON/error schemas need a documented contract. The library should return
those outcomes instead of printing, and classify transport, policy,
persistence, build, activation and recovery failures.

An engine boundary could retain the current `Backend` pattern, but name its
contracts more precisely: `inspect_candidate`, `build_and_snapshot`,
`activate` and `verify_running_identity`. An eligible candidate should bind
application/profile identity and evidence provenance as well as tag/commit.
Before activation, repeat admission and compare the same candidate identity.
Typed evidence is useful for preventing accidental omission; it does not
protect a process that loads an untrusted backend implementation. Only
operator-reviewed installed code may implement privileged boundaries.

No network plugin loader, root shell-command string, release-controlled
configuration, or arbitrary artifact installation callback is needed for the
first interface. Candidate-controlled build scripts remain expected, but
run only as the credential-free builder. A reviewed build adapter can support
a different application without executing its deployment scripts as root.

## Invariants and compatibility work

Preserve these contracts through any refactor:

1. Verify trusted tag and exact release-suite evidence before executing any
   candidate code. API transport failures stop the poll; missing/pending checks
   never admit a tag and are reconsidered. A later failed/pending run revokes
   an older success.
2. Downloader credentials are unavailable to builder/runtime. Root snapshots
   unprivileged inputs without opening attacker-selected source paths itself.
   Only validated regular artifacts with expected identity reach activation.
3. Root-owned durable pending state and the candidate block are synced before
   build/activation. Recovery runs before network access and restores the
   recorded previous release; it must work while credentials are unavailable.
4. Keep the previous artifacts through candidate health and final durable
   commit. Persisted failure blocks survive restart; explicit retry never
   weakens admission. If recovery or its writes fail, retain pending state
   and report the failure rather than claiming restoration succeeded.
5. Every mutating entry point uses the same deployment lock. Configuration,
   state directories, releases, helpers and service identity must not cross
   application boundaries or become writable by builder/runtime accounts.
6. Application releases cannot update the privileged toolkit or its trust
   policy. Toolkit upgrades remain separate operator actions with their own
   signed distribution, compatibility and recovery procedure.

The current JSON state has no explicit schema/profile version and rejects
unknown fields. Extraction needs a versioned state format, migration and
recovery tests, and a way to refuse incompatible installed adapters before
changing anything. Existing transactions must recover with the recorded
layout/service contract. Bind state to deployment/profile identity and detect
policy changes; do not load an interrupted transaction under an unrelated
profile. Decide how an operator changes policy while recovery is pending.
Never discard pending state or silently initialize a fresh deployment to
work around incompatibility. Initial deployment without a known healthy
release remains a separate bootstrap step.

GitHub validation is a provider adapter, not a generic "green check" flag.
The current implementation uses run and attempt-specific job APIs; future
profiles need fixtures for their actual reusable-workflow shape and required
jobs. GitHub documents
[run identity and filtering](https://docs.github.com/en/rest/actions/workflow-runs)
and [jobs for a run attempt](https://docs.github.com/en/rest/actions/workflow-jobs#list-jobs-for-a-workflow-run-attempt).
A transport filter alone is not admission: validate returned repository,
workflow, event, tag ref, commit and current attempt again. A configurable
required-job list must preserve missing/duplicate/skipped-job rejection and
the profile's policy for unexpected jobs.

## Develop inside this repository, then extract

| Stage | Work and evidence needed before moving on |
| --- | --- |
| Establish the reference | Complete issue #6's live upgrade/recovery demonstrations; record disk/service/network failure limits and the installed updater version. |
| Internal prototype, separately authorized | Keep `silkboard-updater` unpublished. Separate engine, GitHub evidence, Linux execution and Silkboard profile modules inside the existing crate. Add validated operator configuration, structured outcomes and state-version handling only with behavior-preserving tests and review. |
| Exercise a second consumer, separately authorized | Choose an application with meaningfully different build/artifact/health requirements. Implement its adapter without copying the engine, adding application-name branches to it, or weakening Silkboard's gates. Repeat end-to-end failure/reboot tests there. |
| Stabilize | Document and test CLI/profile/state/adapter compatibility; demonstrate a toolkit upgrade while a transaction is pending and a supported recovery path. Keep consumer-specific code in profiles. |
| Extract, separately authorized | Move only proven shared components and conformance fixtures into a new repository. Replace workspace/path integration with an explicitly versioned dependency or installed CLI; retain Silkboard's application recipe and acceptance tests here. |

This avoids splitting several packages merely to move files. Separate crates
become useful when a second consumer or compatibility boundary demonstrates
the need; they are not prerequisites for starting an internal prototype.
No extraction date follows from this assessment.

Consider the interface stable enough to extract only after both consumers
have demonstrated admission, waiting checks, source revocation during build,
failed build/start/health, blocked retry, interrupted activation, offline
recovery and reboot persistence. They should use the same documented
interface and pass shared conformance fixtures without private API access.
Require evidence of supported state/config upgrades, retained previous
artifacts, secret isolation, actionable diagnostics and independent security
review. Set an observation window and supported-version policy with the
owner; a second example or a small version number alone is not stability.

Extraction also needs repository/package/CLI naming, ownership, a complete
license/attribution decision, signed releases, installer verification,
security reporting, documentation and a consumer upgrade procedure. The
current crate declares MIT but is `publish = false`; no standalone toolkit
license/distribution contract is established. Do not bundle application
credentials or create a new repository merely to reserve a name.

## Comparisons and unresolved choices

Rechecked the roadmap's local source baselines:

- MuTrack `5bdf415cbae23835c8449cf3ef3747aa8583be8f`,
  [tag updater](https://github.com/breki/mutrack/blob/5bdf415cbae23835c8449cf3ef3747aa8583be8f/deploy/malina/deploy-demo.sh):
  useful timer/build/staging/link-switch pattern. Its root build and tag-only
  admission are not the proposed toolkit's security baseline.
- Phren `effd71bed44dd95fcbe7a4d1f6f609c2b58e05ba`,
  [deployment script](https://github.com/breki/phren/blob/effd71bed44dd95fcbe7a4d1f6f609c2b58e05ba/deploy/malina/phren-deploy.sh):
  useful pre-build signature check and application-specific rendered-output
  verification. Its branch/commit deployment is a different release policy.
- Hoard `79c06892e3eff1e89782b8834813ffdcd1a4b0ed`,
  [pull deployment plan](https://github.com/breki/hoard/blob/79c06892e3eff1e89782b8834813ffdcd1a4b0ed/docs/issues/malina-pull-deploy.md):
  marked planning. It supports a comparison of intended boundaries, not a
  claim that Hoard is a second working pull-updater consumer.

The owner authorized the first in-repository implementation after this
assessment. No second-consumer adoption, shared-project name, public
publication, additional provider/channel, or extraction has been chosen.
Finish Silkboard's live proof before treating the reference as operational,
then evaluate a named second consumer and the remaining stabilization work.
