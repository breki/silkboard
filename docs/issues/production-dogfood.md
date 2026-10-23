# Production agent progress dogfooding (#39)

Goal: finish the initial Codex-only production rollout. GitHub stays the task
tracker; agents publish short observations and never fetch dashboard content.

Owner-approved rollout delivered signed v0.3.0, private state and phone owner
setup/login. Frosti now has the standalone publisher and a privately installed
project-scoped credential. The owner enabled and reviewed native reporting.
Real automatic reports from two independent Codex sessions appear in the
owner's feed/history. Live authorization denials and application restart
persistence passed on 2026-10-19; see the
[durable evidence](../dogfood-rollout.md#evidence-ledger).

Owner scope decision, 2026-10-19: defer independent backups, isolated restore
and application recovery acceptance to
[#52](https://github.com/breki/silkboard/issues/52). These no longer block #39;
no recovery or ongoing backup protection is claimed. Backup destination, keys,
schedule and retention remain undecided. Schema-changing/destructive operations
still require their safeguards and separate approval.

Remaining #39 acceptance:
1. Review and merge the exact live evidence through normal documentation checks.
2. Select the version under the release workflow, prepare the next compatible
   signed release, then obtain exact publication/upgrade authorization.
3. Verify all real existing reports and native identities survive a normal
   tagged automatic upgrade. Retain updater trust/state and schema 3.
4. Verify post-upgrade automatic publishing and owner phone feed/history access,
   including latest messages, distinct sessions and mobile layout.
5. Record exact release-suite/admission/activation and ordinary polling evidence.
   Do not close #39 until the remaining acceptance is actually complete.

No schema or dependency change is proposed. The pending reusable owner-handoff
tooling is repository preparation; releasing it does not install it on this
already initialized server or replace the privileged updater.
