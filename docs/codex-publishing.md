# Automatic Codex progress publishing

The installed [publisher](publishing-cli.md) accepts native Codex hooks through
`silkboard-publish codex-hook --config PATH`. Issue
[#37](https://github.com/breki/silkboard/issues/37) adds automatic observations
for completed turns and child-agent starts/completions. There is no service,
outgoing queue or target Rust/Node requirement. Claude integration remains
[#43](https://github.com/breki/silkboard/issues/43).

## Enable on a development machine

Install the executable and private token/configuration/state directories using
the [publisher guide](publishing-cli.md#private-configuration). Add this optional
block to its existing mode-0600 `config.json`, using actual absolute paths:

```json
"codex": {
  "workspace": "/home/USER/work/src/silkboard",
  "hostname": "development-vm",
  "session_name": "Silkboard development"
}
```

This is a member of the existing configuration object, alongside `endpoint`,
`project`, `token_file` and `state_dir`. Without `codex`, automatic publishing
is disabled. `hostname` is an operator-chosen stable identity for this machine.
`session_name` is optional display text, shared by root sessions using this
configuration; concurrent sessions still have distinct native IDs. Changing
the name preserves identity/counters. Children use native `agent_type` as an
optional readable label. Names do not replace IDs.

The wrapper defaults to `$HOME/.local/bin/silkboard-publish` and
`$HOME/.config/silkboard-publish/config.json`. An operator can set
`SILKBOARD_PUBLISH_BIN` and `SILKBOARD_PUBLISH_CONFIG` before starting Codex to
select different installed files. Native hook input cannot choose these paths,
credentials or the destination. Keep operator configuration outside the checkout.
Configure each separately located worktree/project explicitly. Workspace
matching is lexical: the absolute native `cwd` must be the configured root or
a descendant, without parent traversal. This filter is not a filesystem sandbox;
the operator must choose the intended root and control its paths.

Use Codex 0.162.1, the version verified here, or verify the documented native
hook contract with a newer installed version. The project registrations in
[`.codex/hooks.json`](../.codex/hooks.json) call the thin
[wrapper](../.codex/hooks/publish-progress.sh) asynchronously with a five-second
native timeout. The publisher retains its three-second network budget.
After reviewing the commands, use Codex's `/hooks` interface to trust and
enable the three new reporting handlers. Changed command definitions require
review/retrust. This procedure does not edit user trust records or bypass them.
Keep the existing [validation hook](ai-agents/stop-check-hook.md) and
[Git-summary hook](ai-agents/git-status-hook.md) enabled; their commands,
timeouts, order and handler keys are preserved.

## What is published

| Native event | Identity | Observation |
| --- | --- | --- |
| `Stop` | Native `session_id` | Bounded first sentence, or `Turn completed.` |
| `SubagentStart` | Parent `session_id` plus native `agent_id` | `Subagent started.` |
| `SubagentStop` | Parent `session_id` plus native `agent_id` | Bounded first sentence, or `Subagent completed.` |

The tool is always `codex`. For children, `parent_session_id` is the parent's
native session ID. Resuming a session retains its identity and private counter;
concurrent sessions and children have independent counters. Reporting is an
observation, not a claim that validation passed. Async callbacks can arrive out
of order or be dropped on contention, outages, timeout or session shutdown.

The owner selected a short first sentence from the final reply. The adapter
uses only the first nonblank line of `last_assistant_message`, stops at ASCII
sentence punctuation followed by whitespace/end (allowing Markdown emphasis
markers), and caps the result at 240 UTF-8 bytes without splitting a character.
It trims outer Markdown markers, removes controls and bidirectional overrides,
and falls back to lifecycle text for absent/empty replies or opening code fences.
This is a simple excerpt, not an AI summary or secret detector. Keep final
replies suitable for the private dashboard; abbreviations may shorten excerpts.

Only the fields in the table, `cwd`, `agent_type`, `stop_hook_active` and the
optional final reply are interpreted. Native stdin is capped at 64 KiB;
oversized/invalid payloads are dropped. Unknown fields, prompts, tool data,
transcript paths and transcript contents are ignored. Nothing follows those
paths or uploads repository files, handoffs, command logs or credentials.
Recursive Stop callbacks (`stop_hook_active: true`) are skipped. Session-start,
session-end and individual tool events are not registered for publishing.

The documented hook invocation always exits 0 with `{}` and no stderr,
including disabled/missing configuration, invalid input, revoked credentials,
busy counters and delivery failure. Missing installations are a quiet no-op.
It emits no block decision, permission, instructions or added context to Codex.
Native async execution keeps delivery off the triggering turn's critical path;
it does not weaken or replace the separate synchronous validation hook.

## Verification and rollout boundary

`cargo test -p silkboard-publish` covers payload privacy/limits, first-sentence
selection, root/child/resume identities, private state, wrapper failure and
actual HTTP delivery. No Codex installation or model account is needed for CI.

On 2026-10-18, an isolated native test also used installed `codex-cli 0.162.1`,
its generated app-server schema and a local controlled Responses provider.
An owned temporary `CODEX_HOME` and self-authored capture/publisher handlers
were discovered as untrusted, then trusted by their reviewed native hashes
only in that temporary configuration. No real account or user hook trust was
changed. The native model fixture drove actual child creation and final replies.
The compiled publisher ran with an empty executable search path and sent to a
disposable Silkboard backend using a locally issued project credential.

Read-only SQLite inspection confirmed five events: two root observations with
sequences 1/2 across an owned app-server restart and native resume, two child
observations with sequences 1/2 and the actual parent/child IDs, and an independent
concurrent root at sequence 1. Only the first reply sentence was stored. Captured
native `SessionStart` metadata independently identified `startup` and `resume`;
the adapter ignored those probe-only events. Every callback returned `{}`,
exit 0 and empty stderr. A Stop callback held at a local gate was still pending
when the native `turn/completed` notification arrived, proving async behavior.

The final native fixture used root ID
`01a14f7c-dad5-73d0-8745-d5009d2e5d5c` and child ID
`01a14f7c-db27-7011-a5da-d7bffbceb928`; these are disposable test identities.
This verifies the installed native runtime with controlled model responses and
authenticated local publishing. It is not a hosted-model or production dogfood
run. Separately, on 2026-10-19, the Frosti installation built from signed
v0.3.0 automatically sent real observations from two independent native Codex
sessions to production. The owner confirmed distinct sessions and supplied a
feed/history screenshot. The [rollout evidence](dogfood-rollout.md#evidence-ledger)
separates this live acceptance from the controlled native fixtures and the
remaining data-bearing upgrade check in #39.

Native semantics were checked against OpenAI's
[hook documentation](https://learn.chatgpt.com/docs/hooks),
[custom provider configuration](https://learn.chatgpt.com/docs/config-file/config-advanced)
and [Responses streaming reference](https://developers.openai.com/api/reference/resources/responses/streaming-events),
alongside actual installed schemas/payloads. Recheck them when upgrading Codex;
do not infer native metadata from transcript files.

## Explicit evolving task plans

The installed publisher also accepts `codex-plan {start|publish} --config PATH`
with a complete graph on stdin. It derives reporter identity from the existing
operator `codex` configuration and native `CODEX_THREAD_ID`; the graph does not
include `reporter`. This requires an explicit `"plan_reporting": true` member
inside `codex`. It defaults to false and does not affect existing summaries.
Install a server supporting #75 and a compatible publisher before enabling it;
these plan features have not yet been released to production.

Follow the [Codex plan workflow](ai-agents/codex-plans.md) for stable identities,
actual step transitions, revisions, failures and explicit session handoff.
Existing native hook definitions remain unchanged and need no new trust grant
for this explicit command. Calls remain subject to the agent's normal shell
permissions and operator configuration. There is no transcript parser, new
hook event, app-server listener or background delivery process.

### Local plan integration evidence

On 2026-10-24, installed Codex 0.163.0 generated its actual app-server schema
and ran an owned native shell probe against a controlled local provider.
`CODEX_THREAD_ID` matched the native app-server thread ID. The probe used only
an owned temporary home and self-authored commands; it did not contact a real
model account, read transcripts or change the user's hook trust.

Separately, the Codex agent developing #77 used an installed copy of the new
publisher to send its actual current task graph to a disposable local Axum
instance, using that session's native environment identity. The instance had
its own initialized owner, project credential, private state and SQLite file.
Start and a subsequent full update were acknowledged; read-only fixture data
inspection confirmed two snapshots, sequence 2, definition revision 1 and the
actual native reporter identity. Adapter/workflow steps completed while the
verification step became active. The CLI ran without runtime helper commands.
These are local development observations, not production or phone acceptance.

Compiled CLI/API tests additionally exercise blocked/done/reopened steps,
revised definitions, multiple active steps, explicit session handoff and
rejection of the old writer. A Stop observation leaves the task snapshot
unchanged. Future UI, combined freshness/recovery checks and production
rollout remain #78–#81.
