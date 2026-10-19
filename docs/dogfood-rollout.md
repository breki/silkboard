# Initial Codex production dogfood rollout (#39)

Status on 2026-10-19: reviewed preparation merged; signed v0.3.0 published and
healthy in production at `46c28d66200b0b13115abfd45744cf58e4d83730`.
Private persistent storage is enabled and the owner confirmed phone registration
and login. Publisher installation, real Codex/feed acceptance and live data
recovery remain pending. The procedure below retains the phased boundaries.

## Verified rollout so far

- Exact v0.3.0 release suite [39189107503](https://github.com/breki/silkboard/actions/runs/39189107503)
  passed; the installed updater admitted and activated that exact revision.
  Polling was restored and an ordinary scheduled poll succeeded. The previous
  v0.2.2 release and v0.2.1 block were retained; no pending transaction remained.
- Explicitly approved state activation installed only the reviewed application
  drop-in and restarted the app. Directory 0700/database 0600, healthy exact
  identity, unauthenticated owner-read refusal and static private-path refusal
  were verified. Service mount-namespace/UID checks denied updater paths and
  release writes; privileged updater files and units remained unchanged.
- The owner selected a Tailscale-identity-restricted temporary phone handoff.
  Live checks admitted the owner, rejected guest-local/spoofed requests, and
  verified that stopping removed only the extra listener. A child UID-switch
  failure was reproduced with a disposable database; explicit ambient CAP_SETUID
  fixed it without relaxing the sandbox. The child ran as the app user with
  zero effective/permitted/ambient capabilities and NoNewPrivileges preserved.
- The owner confirmed successful phone registration and login. The setup API
  reported closed. The consumed temporary handoff was removed; port 443,
  healthy v0.3.0 and the existing updater/timer were verified afterwards.
  The [reusable helper](../deploy/owner-handoff/README.md) preserves the tool
  for future provisioning; that generalized version is not installed here.

No publisher token/client/hooks were installed by this activation. No recurring
backup policy, independent snapshot, data-bearing tagged upgrade or restore
rehearsal is claimed. Phone login is owner-reported; feed acceptance with real
reports remains outstanding. Private operator records retain invocation/state
and checksum evidence without credentials or application contents.

## Prepared scope and authorization

Deliver the agreed publish-only model on the current development setup:
SQLite progress, phone owner setup/login/feed, owner token management,
standalone publisher and automatic native Codex reports. GitHub remains the
task tracker. Claude #43, Bombyx migration #13 and reboot recovery #18 are
independent. No daemon, outgoing queue, handoff upload or dashboard task fetch.

Implementation includes the [opt-in state drop-in](../deploy/vm/silkboard-progress.conf),
[private snapshot/recovery commands](application-data.md) and aligned canon.
Normal [review/commit](ai-agents/catalog.md) delivers this preparation as a
reviewed draft PR. Keep #39 open until the live checklist is satisfied.

Before any publication/change, present the owner a concrete exact-revision
release and rollout proposal and obtain the authorization required by the
[release workflow](ai-agents/workflows/release.md). Preparation alone does not
authorize merge, tag publication, service changes, password/token installation,
hook trust edits or disruptive tests. Existing approved SSH and signing routes
can be reused; do not copy GitHub credentials into the application.

## 1. Review and release

1. Complete full local validation, ARM64 lint and sequential independent
   reviews of the final preparation. Record exact head and CI results.
2. Obtain merge authorization and merge only the reviewed CI-green head.
3. Ask the owner to choose the release version. This adds user-visible
   functionality since v0.2.2, so a minor release is the normal recommendation;
   do not invent or publish a tag here. Prepare separate release bookkeeping,
   full gates/standalone audit and a locally verified signed annotated tag.
4. Record and seek approval for the exact publication and live rollout.
   Publication can trigger the existing automatic updater. For the controlled
   initial rollout, stop the polling timer only when authorized, wait for any
   active update to finish, and record its enabled/active state plus pending,
   blocks, current identity and protected installed unit checksums. Do not kill
   an active update or change unrelated host services.
5. Publish the approved signed tag through approved Git SSH. Require the
   selected workflow 387356305 and **all three exact-tag/exact-commit jobs**
   to succeed; a tag or PR CI alone is insufficient. Independently read back
   the installed updater's admission result. Let the normal installed updater
   build/stage/activate the reviewed application under its existing policy.
   Do not replace the privileged updater with candidate code.
6. Confirm exact new identity and health with both loopback and private HTTPS
   checks while the service still has no data drop-in. Record retained previous
   release and updater state. On failure use the existing updater recovery;
   no state directory has been introduced yet.

## 2. Enable private state and first-owner registration

After explicit service/credential rollout approval and a verified compatible
release is installed:

1. Save the current unit/drop-ins, release identity, timer state and recovery
   instructions in a protected operator record. Refuse to overwrite an existing
   `20-progress.conf`; inspect any existing state before proceeding.
2. Install the reviewed drop-in root-owned mode 0644 at
   `/etc/systemd/system/silkboard.service.d/20-progress.conf`. Use
   `systemd-analyze verify silkboard.service`, `systemctl daemon-reload`, then
   restart only the application. Check healthy exact identity, effective
   environment/StateDirectory, mode0700 directory and mode0600 database.
3. Verify the service cannot read `/etc/silkboard/updater-token`, fetch/build/
   updater state or write release binaries. Exercise the actual unit sandbox,
   not just a shell with the same UID. Verify database, journals and exports
   cannot be downloaded through static routes; unauthorized owner reads fail.
4. Confirm `/api/auth/setup` is waiting and unauthenticated owner reads are401.
   An uninitialized owner must keep publisher ingestion503; a health200 alone
   does not establish dashboard readiness.
5. Prepare the [one-time setup link](owner-setup.md#provisioning-and-private-delivery)
   with the installed binary and the configured database/origin as `silkboard`.
   Capture it directly into an operator-private pipe/file outside the checkout;
   never through tool/chat/log output or a guest raw-link file. Agree a private
   phone-accessible delivery channel **before generating it**. It expires after
   one hour. Preparing/replacing a link does not register the owner.
6. The owner opens that private link on the phone, sets their own password,
   signs in and confirms the owner page. Do not invent successful phone input,
   set a password for the owner or ask for any password/token in chat.
7. The owner uses [Manage publisher credentials](publisher-tokens.md) to issue
   a unique project `silkboard` installation credential and save it privately.
   Agree secure transfer to the publishing machine; do not reuse updater
   credentials or install an owner browser session on the development client.

If delivery fails, keep ingestion closed or revoke the affected installation
and prepare a new label/link as appropriate. Do not bypass first-owner setup.

## 3. Install reporting and verify real work

1. Build/install the reviewed `silkboard-publish` executable for this development
   machine, separately from the ARM64 server build. Record its package version,
   source commit and checksum. The current publisher package is0.1.0; record
   the actual release revision rather than assuming it matches server version.
   No target Rust/Cargo dependency, daemon or durable queue is introduced.
2. Privately install the token, mode0600 configuration and mode0700 counter
   directory according to [publishing](publishing-cli.md). Use the fixed
   production HTTPS endpoint/project, actual workspace and stable hostname.
   Configuration and credentials stay outside the repository.
3. Use the verified native Codex hook contract in [integration](codex-publishing.md).
   Review the three asynchronous reporting commands and obtain the owner's
   normal hook trust through Codex, preserving existing validation/Git-summary
   handlers. Do not edit trust files or run untrusted hooks by inference.
4. Observe actual ordinary work from two independently identified Codex sessions,
   or a root and native child, with short suitable replies. Verify automatic
   progress appears with correct project/host/tool/native/parent identity and
   separate latest messages/history. Fixtures and manually fabricated hook JSON
   are not production integration evidence. Delivery remains best-effort.
5. Ask the owner to confirm phone login, distinct latest messages, history and
   no horizontal overflow. Verify no-cookie and publisher-only reads401,
   wrong-project publishing401 and a deliberately revoked test credential401,
   without printing any credential or recording sensitive browser bodies.

## 4. Persistence, independent snapshot and recovery rehearsal

Agree the encrypted off-machine backup destination, snapshot naming and
operational backup frequency/retention before considering ongoing protection
established. The initial approved evidence includes:

1. Record nonsecret canary identity/counts and owner/publisher validity; restart
   the application and confirm the same observations/authentication survive.
2. Take a fresh [consistent snapshot](application-data.md), pull it privately
   off the VM into the agreed protected storage, and record only its checksum,
   schema and counts. The app's own export directory is not independent backup.
3. Restore that copy to an isolated new private directory with no production
   listeners/hooks. Verify history/password retention, revoked publisher tokens,
   cleared browser sessions/setup links and no change to the source/live DB.
   Remove the rehearsal copy through approved cleanup after saving safe evidence.
4. Prepare and approve a second compatible patch release if needed to demonstrate
   a real tagged automatic upgrade with **existing data**. The initial upgrade
   from status-only v0.2.2 predates state creation and cannot prove this. Avoid
   fabricated feature changes solely for a version bump; release evidence/docs
   can supply legitimate follow-up bookkeeping. Verify exact release suite and
   automatic activation, then unchanged canaries, owner login and publisher
   identity/counters. No schema change is proposed in this initial rollout.
5. Restore the original timer enabled/active state, observe an ordinary scheduled
   poll, and verify healthy exact release/state and only expected drop-in changes.
   Keep v0.2.1 blocked and no pending transaction; do not reset updater history.

## Recovery before completion

If enabling state fails, preserve the new data privately, stop the app, remove
only this rollout's data drop-in and reload/restart the known status-only
configuration. Returning to v0.2.2 cannot provide a feed, and must not be
reported as preserving dashboard availability. Keep data for diagnosis; never
remove it to make a startup health check pass. Pause polling and use the
installed updater's existing recovery procedure if activation itself failed.

If credentials are compromised/lost, use documented owner reset or publisher
revocation/reissue. Do not restore an old backup merely to retrieve a token.
Production data replacement requires a separately approved stopped-service
recovery using new directories, the compatible release and fresh credentials
as described in the data guide. Never copy a main DB file over old sidecars or
assume an application symlink rollback reverses a schema migration.

## Evidence ledger (pending live execution)

| Requirement | Evidence required | Current state |
| --- | --- | --- |
| Reviewed release | Signed tag, exact commit, three successful release jobs, updater admission | Pending |
| Service confinement | Effective unit, private paths, live denied-access checks | Pending |
| Owner setup | Owner phone registration/login; no secret in records | Pending |
| Real reporting | Installed client/Codex versions, independent native identities, automatic observations | Pending |
| Phone feed and read denial | Owner confirmation/history and unauthorized/publisher401 | Pending |
| Restart and data upgrade | Retained nonsecret canaries before/after actual restart/tagged upgrade | Pending |
| Independent recovery | Protected snapshot checksum and isolated restore results | Pending |
| Normal polling restored | Exact healthy identity, original timer state, scheduled poll, no pending work | Pending |

Local tests and reviews must be recorded separately from this ledger. Once live
acceptance is actually complete, update these source documents, link observed
release runs, record remaining limits and only then close #39 through an
authorized reviewed evidence change. Do not claim completion after preparation.
