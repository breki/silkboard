# In-repository deployment toolkit

Issue [#14](https://github.com/breki/silkboard/issues/14). The owner authorized
this first implementation after the [feasibility assessment](deployment-toolkit-feasibility.md).
The toolkit remains inside the unpublished `silkboard-updater` crate. It has
not been installed on the production guest, extracted or published separately.
Silkboard remains the only registered installed application adapter.

## Library boundaries

| Module | Implemented responsibility |
| --- | --- |
| [engine](../crates/silkboard-updater/src/engine.rs) | Stable release selection, transaction/block state, trusted `Backend` contract, post-build readmission, recovery and explicit retry. No terminal output or direct OS calls. |
| [outcome](../crates/silkboard-updater/src/outcome.rs) | Typed admission, poll results and boundary failures, including a separate restored-release or failed-recovery result. |
| [profile](../crates/silkboard-updater/src/profile.rs) | Validated JSON admission policy with mandatory repository/workflow identity, reusable CI path and a nonempty exact successful job set. |
| [evidence](../crates/silkboard-updater/src/evidence.rs) | Policy-parameterized GitHub checks. Compatibility functions retain the canonical Silkboard policy. |
| [storage](../crates/silkboard-updater/src/storage.rs) | Versioned state encoding, exact policy/adapter binding, validated legacy reads and refusal of incompatible state. No filesystem writes in the codec. |
| [cli](../crates/silkboard-updater/src/cli.rs) | Parsed requests, operation dispatch, versioned JSON and compatibility text. A caller must provide serialization and a trusted backend. |
| [linux](../crates/silkboard-updater/src/linux.rs) | The installed Silkboard Linux adapter: account/credential separation, protected snapshots, local builds, durable files, current-link swap, systemd and exact-identity health. |

The facade retains existing imports such as `Backend`, `Release`, `poll` and
`retry`. Poll/retry now return typed results/errors; this is an experimental
in-repository Rust API, not a stable published SDK. `Release` validates an
identity, not authorization. A trusted backend must verify both gates before
returning eligible admission. The engine also rejects an admitted tag that
differs from the requested tag and rechecks eligibility before activation.

A new application can reuse the engine and evidence evaluator through its
own reviewed `Backend`, with different build/artifact/health behavior. That
code executes with the operator's authority and must preserve its documented
contracts. It is not loaded from candidate source or a network plugin.
There is no generic privileged shell-command field or dashboard deployment
endpoint. Provisioning remains the existing application/operator procedure.

A second policy fixture exercises different repository IDs, workflow paths
and jobs. It is a test fixture, not a second deployed consumer or evidence
that extraction criteria have been met. No new dependency versions are needed.

## Operator policy

The Silkboard adapter reads only `/etc/silkboard/deployment.json`; command
arguments cannot select an arbitrary file. The
[shipped JSON example](../deploy/vm/deployment.json) preserves the agreed
repository/workflow IDs, paths and all three required jobs. Its schema is 1.
Unknown fields/schemas, zero IDs, unsafe names/paths, empty/duplicate job
sets, and invalid JSON fail before backend work. The file and its parent
must be root-owned, non-writable by group/others and not symlinks. Policy
size is limited to 64 KiB.

Absence uses the compiled canonical Silkboard policy for legacy compatibility.
An existing insecure policy directory is an error even if the file is absent.
No file or field disables signature verification or GitHub gates. The signer
file remains independently installed at `/etc/silkboard/release-signers`;
credentials remain outside the policy. The installer supplies the example
only when no policy path exists and preserves an existing operator policy.

The library accepts other validated admission policies, but the current
Linux adapter and installed CLI register only the `silkboard` application.
They retain its account names, protected paths, build/staging helpers,
service and health schema. Changing a JSON application name cannot install
an arbitrary application or root helper. Additional real adapters require
reviewed code and a separately authorized installation/adoption.

All required jobs must succeed exactly once, with no unexpected jobs, and
must belong to the currently checked run attempt. Repository/workflow/tag/
commit/reusable-reference checks and the final run readback remain mandatory.
The downloader's Git/API URLs derive from the validated repository, not a
caller-supplied URL. Existing one-argument API-helper calls remain supported.
Changing a policy grants no new GitHub token permissions.

## Installed commands

After an operator installs this reviewed version and its matching helpers:

```sh
sudo deployctl status silkboard
sudo deployctl inspect silkboard v0.1.1
sudo deployctl poll silkboard
sudo deployctl retry silkboard v0.2.0
```

The new [deployctl entry point](../deploy/vm/deployctl.sh) accepts only the
registered profile and routes every operation through the existing
`/usr/local/sbin/silkboard-update` wrapper. It uses the same lock, five-second
lock wait and temporary downloader credential lifecycle as timer/operator
compatibility commands. The timer still calls `silkboard-update poll` every
ten minutes. Never call the raw `/usr/local/libexec/silkboard/updater` helper
as an operator interface: it assumes the wrapper owns serialization and token
lifetime. Root can bypass these contracts; the CLI is not a root sandbox.

The existing commands remain supported, including human-readable output and
the old status object's `blocked`/`pending` fields. They can opt into the new
report format with `silkboard-update --json ...`. `--profile silkboard` is an
explicit selector for the existing registered adapter, not a path.

JSON reports have top-level `schema: 1` and `outcome`. A poll's `result`
distinguishes `unchanged`, `activated` and `recovered`. An unchanged result
includes newer tags classified as waiting, rejected or unclassified by a
compatibility backend. Inspection reports eligibility without building or
activating; retry clears only a valid named block. Status reads local state
without contacting GitHub and does not claim application health.

Errors include a boundary `stage` and diagnostic, with candidate/restored/
failed-recovery fields when applicable. Build output goes to stderr/journal,
keeping the machine report on stdout. A successful command exits 0, including
JSON inspection that successfully determines a tag is waiting or rejected;
read `admission` before treating it as eligible. Backend/parse failures exit
nonzero. Legacy human inspection still fails for an ineligible tag. Launcher,
lock, signal or process-start failures may have only stderr and no JSON;
callers must check exit status and must not infer success from absent output.

## State compatibility and installation

State saves now write a schema-1 envelope containing adapter
`silkboard-linux-v1`, the complete validated admission policy and the existing
transaction/block state. This binds JSON policy, not the contents of the
separately installed signer file or an updater binary hash. Signer/toolkit
changes remain deliberate operator actions; recovery uses retained previous
artifacts without fresh network admission, as before. Loading rejects unknown
fields/schemas, invalid
transactions, policy mismatch and adapter mismatch. It does not discard,
rewrite or reset a mismatched pending transaction. Exact policy changes,
including job ordering, require an explicit migration plan.

The old unversioned state is accepted only with the canonical Silkboard
policy and adapter. A read/status operation does not migrate its bytes. The
next save writes the envelope. A pending legacy transaction first restores
the recorded previous release without network admission, then persists its
remaining block with the policy binding. Local filesystem tests cover this
path. Changing policy while legacy recovery is pending is refused.

Installation remains an explicit operator action, separate from application
release updates. Use [the updater installation procedure](automatic-upgrades.md)
with the reviewed binary and the complete matching `deploy/vm/` files,
including `deployctl.sh` and `deployment.json`. Before upgrading installed
privileged code, stop the timer, let any service finish, inspect pending
state, and save a protected backup of the state/policy and previous installed
binary/helpers. Verify status and known-release admission before re-enabling
the timer. No application release replaces these operator-owned files.

This reader supports the old state and schema 1; the old installed updater
cannot read the new envelope. Rolling privileged code back therefore needs
a compatible state reader. Do not restore a pre-upgrade state backup after
new transactions have begun: that can lose a pending recovery or failed-tag
block. Preserve evidence and use the matching reviewed reader/policy to finish
recovery before designing a migration. There is no arbitrary schema migration,
policy-reset, forced downgrade, pruning or privileged self-update command.

The adapter's paths/service/layout must remain compatible with its recorded
identity. Changing that contract requires a new adapter identity and a tested
operator migration; changing only the string to silence a mismatch is unsafe.
Application data migrations and their rollback remain application-specific.

## Verification and remaining limits

Behavioral tests cover typed outcomes and failures, independent policy
fixtures, malformed policy, unsafe policy files, exact GitHub evidence,
state mismatch and legacy recovery, CLI validation and the existing build/
health/retry/recovery paths. Shell fixtures exercise registered dispatch and
credential transport without real tokens. These checks are local; they do
not establish a second production consumer or a completed live rollout.

The installed production updater remains the previously reviewed version.
Issue #6 still owns live newer-release upgrade, rollback/retry and updater
reboot proof. Interface stabilization, a named second consumer, an observation
window, extraction, naming/licensing/distribution and other platforms remain
future decisions. Implementing the initial interface does not satisfy those
extraction criteria or expand later product milestones.
