# Task-plan combined acceptance

Local acceptance record for [#80](https://github.com/breki/silkboard/issues/80),
on 2026-10-25, after #73–#79. Application source baseline:
`74b1d039597b93ee06e49f6864ce1e916e9e5346`. The added acceptance fixture changes
no application schema, reporting permission or production configuration.
[Release and real phone dogfooding](task-plan-rollout.md) are recorded
separately, including the remaining acceptance. Reported task completion does
not verify checks or authorize merge, release or execution.

## Acceptance matrix

| Requirement | Evidence | Result and limit |
| --- | --- | --- |
| Same feature as a branching task; all four states; evolving definition | [Combined browser case](../frontend/e2e/acceptance.spec.js) | Installed Codex adapter sends a clearly labeled task-plan feature fixture; graph shows pending/active/done/blocked, a new recovery branch and revision 2 |
| Step selection, Markdown, current and historical views | Same combined case and [graph cases](../frontend/e2e/plans.spec.js) | Owner selects reports/prerequisites/reason; exact earlier snapshot retains old states; returning to Current plan restores latest |
| Existing data and process restart | Combined case | Actual backend stops/restarts against one private SQLite file; owner login, publisher token, feed canary and exact plan/history survive; subsequent installed publisher update is accepted |
| Phone, desktop, keyboard/focus, arrows and large graphs | Graph cases and combined case | Desktop and 320px phone, modal Escape/focus return/removal, readable directed arrows, ordered alternative, no page overflow, maximum 128 nodes/512 edges |
| Untrusted text and read/write authority | Combined case, graph cases, [route tests](../crates/silkboard-web/tests/plans.rs) | No loaded images or executable markup; publisher-only owner reads, wrong-project writes and oversized requests denied; owner cookie alone does not publish |
| Graph validity, request/counter bounds and idempotency | [Plan validation](../crates/silkboard-plan/tests/protocol.rs), route tests and [CLI tests](../crates/silkboard-publish/tests/plans.rs) | Invalid graphs/counters rejected; accepted snapshot identity and original receipt preserved |
| Missed/delayed reporting and writer replacement | [Recovery fixture](../crates/silkboard-publish/tests/plan_recovery.rs) and [recovery record](task-plan-recovery.md) | Complete later report repairs current state; exact received history remains incomplete; old writers/conflicts cannot regress it |
| Native Codex tool environment | Native probe described below | Installed 0.163.0 supplies the actual thread ID to its shell; version-specific observation, not a promised environment API |
| Actual development reporting | Developing session observations below | Genuine current plan sent through installed compatible publisher; distinct from controlled model responses and synthetic fixtures |
| Existing data upgrade and isolated restore | Compatibility drill below and [persistence tests](../crates/silkboard-web/tests/plan_persistence.rs) | Signed v0.6.0 data opens with additive component; retained old code still serves ordinary feed; restore retains plan data and revokes authority |

The automated identities, states, gate reasons and native environment override
in the combined browser case are fixtures. They are not an actual Codex session
or evidence that production work has completed. The case runs the compiled,
installed executable rather than replacing publishing with an HTTP stub.
It owns its local listeners, processes, credentials and storage, and adds no
application test-control endpoint.

The first run exceeded the browser's setup-hook budget while building the CLI.
Both web and publisher binaries now build in the existing fixture startup step,
using the fresh normal test profile and locked inputs before browser hooks run.
The hook timeout was not weakened; the corrected combined case passed.
Full configured validation, cross-target lint and independent reviews are
required in the implementation PR, beyond these targeted observations.

## Native observations, separate from fixtures

Installed `codex-cli 0.163.0` was checked against the official
[app-server protocol](https://learn.chatgpt.com/docs/app-server) and
[custom-provider configuration](https://learn.chatgpt.com/docs/config-file/config-advanced).
The owned probe initialized app-server, started a thread/turn and let the actual
native shell tool invoke an installed `silkboard-publish codex-plan start`.
Its local provider returned only self-authored controlled responses; no real
model account or user transcript was read. Its private child home/configuration
and workspace did not change the user's hooks, trust or installed settings.

The received five-step graph contained done/active/active/blocked/pending states.
Read-only inspection of the disposable database confirmed its reporter's
native session ID equaled the actual app-server thread ID; the native shell
identity file independently matched it. Tool completion and server receipt,
not the final assistant sentence alone, establish this probe's success.
This proves the native integration path with controlled model output. It does
not prove an autonomous hosted model will choose the correct plan updates.

Separately, the real Codex session developing #80 used a compatible publisher
installed into a private verification directory and an explicitly enabled,
fixture-only operator configuration. Its actual current plan reports described
completed inspection/combined checks and the ongoing native/evidence work.
The first acknowledgement and database snapshot matched its native environment;
subsequent meaningful boundaries use the same task and private working state.
These are actual development observations, not simulated model responses.
No production token, installation or plan-reporting setting was used or changed.

`CODEX_THREAD_ID` remains a measured 0.163.0 integration detail. Recheck actual
runtime behavior when upgrading Codex; do not infer identity from transcripts,
reuse a previous session or fall back to another environment variable.

## Existing-data and recovery drill

The retained v0.6.0 application was built from clean signed source
`07026bbd0d6a1feff9d04a4e47856d1611afa00d`; the tag signature was reverified.
Its binary SHA-256 was
`750962ca8d8872b59b8cb53ecf41243fc670c12da6437a387505f663ea4ae510`.
The isolated drill:

1. Created owner/publisher state and an ordinary progress canary with that old
   executable. Read-only inspection confirmed no `plan_schema` component.
2. Opened the same private data with current code, published a plan and signed
   in. Base schema remained 3 and the additive plan component was version 1.
3. Ran retained v0.6.0 against that data, checked its exact status identity,
   published another ordinary report and read both through the owner feed.
4. Used the retained operator backup/restore commands in isolation. New code
   read the retained plan; the restored copy retained plan/owner data but
   rejected the former publisher token and browser cookie.

This exercises real processes and a data-bearing old revision, not just a
synthetic schema or symlink switch. It does not make the old binary capable
of validating future plan-component schemas. Prefer a current compatible
binary for operator snapshots after plan support is installed. The drill
is not a production rollback, independent backup or accepted recovery policy.

## Release handoff for an existing installation

Before #81, use the [release procedure](releases.md),
[private data guide](application-data.md) and
[API compatibility notes](task-plan-api.md#storage-and-compatibility):

- Verify the actual installed revision/schema and a fresh independent private
  snapshot, with compatible recovery artifacts. These local copies do not
  settle #52's destination, encryption, cadence or retention decisions.
- Admit a reviewed signed stable tag only after its exact required release
  workflow passes. Preserve installed updater policy and health/rollback gates.
- Expect additive table initialization, not a base-schema bump or owner reset.
  Recheck existing feed/history, owner login, publisher scope and private modes.
- Install the compatible standalone publisher separately, preserve its private
  working state and explicitly enable plans for the agreed workspace. Existing
  short-report hooks and their trust do not implicitly enable task plans.
- Perform genuine reporting and owner phone acceptance through private HTTPS.
  Record observations without treating a reported done state as verified work.

If recovery is needed, preserve the live data and use the separately authorized
procedure. A code rollback is not a database downgrade; restoring a snapshot
revokes publisher/browser authority and requires private reissuance. None of
those live operations was performed for #80.
