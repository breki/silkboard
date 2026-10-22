# Initial Codex production dogfood rollout (#39)

Status on 2026-10-19: reviewed preparation merged; signed v0.3.0 published and
healthy in production at `46c28d66200b0b13115abfd45744cf58e4d83730`.
Private state, owner phone setup/login, real Codex reports from Frosti,
independent session/feed/history acceptance, live access denials and application
restart persistence are verified. A tagged upgrade with existing data and
post-upgrade phone acceptance remain #39. The owner deferred independent backups,
isolated restore and application recovery acceptance to
[#52](https://github.com/breki/silkboard/issues/52); those no longer block #39.
The procedure retains its authorization boundaries.

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
- Subsequent owner approval installed the Frosti publisher built from signed
  v0.3.0, package 0.1.0, SHA256
  `1604e65265cee264d6c1aba26cd3dfa8983ca509de852a29c73ffbd69a5ae726`.
  A project-scoped token was issued via approved SSH and transferred directly
  to its private client file; the guest raw transfer copy was removed. Config/
  token are mode 0600, configuration/state directories mode 0700. No credential
  value is included in chat, source or evidence. Native Codex 0.162.1 reporting
  was enabled after the owner's explicit approval and native hook review.
- Real automatic reports arrived from two independent root native sessions on
  Frosti. The owner supplied a feed screenshot with a real reply excerpt and
  expanded history, then confirmed seeing both sessions. At the later restart
  baseline production held seven reports across those two sessions. No fabricated
  payload was used for reporting acceptance. Publishing remains best-effort;
  native root/child/resume fixtures are separate evidence.
- Private HTTPS checks returned 401 for project/session/history/publisher metadata
  reads without credentials and with the publish-only token. Wrong-project
  publishing and a dedicated, already revoked test credential also returned 401.
  The test label remains revoked; its raw token and temporary directory were
  removed. No denial-check message was accepted.
- A brief application restart at 07:09 UTC changed the service PID from 29514
  to 31929 and returned healthy exact v0.3.0. All seven prior event metadata
  records and both native session records were retained exactly. Owner presence,
  browser-session count, publisher/revocation metadata and schema 3 were unchanged.
  Setup stayed closed; release path, state drop-in and updater binary checksum
  were unchanged, and polling stayed enabled/active. This verifies stored
  metadata retention, not fresh owner password authentication. The private
  guest record is `/var/log/silkboard-dogfood-restart.4TiSpF/evidence.json`.

No recurring backup policy, independent snapshot, data-bearing tagged upgrade
or restore rehearsal is claimed. Private operator records retain safe checksums,
counts and invocation/state evidence, without credentials or message contents.

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
   `/etc/systemd/system/silkboard.service.d/20-progress.conf`. Before restarting,
   also configure the exact deployment HTTPS origin in an operator-owned
   `21-owner-origin.conf` as described in
   [service storage](application-data.md#service-storage); the tracked template
   supplies no origin. Use
   `systemd-analyze verify silkboard.service`, `systemctl daemon-reload`, then
   restart only the application. Check healthy exact identity, effective
   environment/StateDirectory, mode 0700 directory and mode 0600 database.
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
2. Privately install the token, mode 0600 configuration and mode 0700 counter
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
   no horizontal overflow. Verify no-cookie and publisher-only reads 401,
   wrong-project publishing 401 and a deliberately revoked test credential401,
   without printing any credential or recording sensitive browser bodies.

## 4. Persistence across restart and tagged upgrade (#39)

1. Record existing event/session metadata and owner/publisher presence; restart
   only the application and confirm the same stored observations survive.
   This passed on 2026-10-19 with seven real reports and two native sessions.
   Do not read credential hashes or treat record counts as fresh authentication.
2. Prepare a compatible reviewed release to demonstrate a real tagged automatic
   upgrade with existing data. The initial status-only v0.2.2 to v0.3.0 upgrade
   predates state creation and cannot prove this. Use legitimate pending changes
   and reviewed evidence; obtain the owner's version and exact publication
   decisions through the release workflow. No schema change is proposed.
3. Capture a fresh baseline of existing event/session metadata before activation.
   Verify the exact release suite, admission and automatic activation, then that
   every baseline observation/native identity remains intact. Verify new real
   automatic reports arrive and the owner can still use phone login/feed/history
   with independent latest messages and no horizontal overflow. Do not require
   message counts to stay fixed while ordinary sessions continue publishing.
4. Verify the original polling timer state, an ordinary scheduled poll, healthy
   exact identity and no pending transaction. Keep v0.2.1 blocked and do not reset
   updater history. Preserve the private state drop-in and updater binary.

## 5. Independent backups and recovery (#52, deferred)

The owner deferred independent backups, isolated restore and application
recovery acceptance from #39 to [#52](https://github.com/breki/silkboard/issues/52)
on 2026-10-19. These no longer block #39; no ongoing protection is established.
Destination, encryption/key custody, frequency and retention remain undecided.
Follow the [data guide](application-data.md) when those choices are approved:
create a fresh consistent snapshot, protect an independent encrypted off-VM
copy, restore into a new isolated private location and verify retained data/
owner state and intentionally invalidated credentials without changing production.
The application export directory alone is not an independent backup. Future
schema-changing or destructive operations still require their approved safeguards.

## Failure handling (recovery execution remains separately approved)

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

## Evidence ledger

| Requirement | Observed evidence | Current state |
| --- | --- | --- |
| Reviewed release | Signed v0.3.0, exact commit 46c28d6, three successful release jobs in run 39189107503 and installed updater activation | Verified |
| Service confinement | Effective unit/private paths and live namespace/UID denied-access checks from state activation | Verified |
| Owner setup | Owner confirmed phone registration/login; setup permanently closed | Verified |
| Real reporting | Released publisher package 0.1.0, Codex 0.162.1, two independent native root sessions and real automatic observations | Verified |
| Phone feed/history | Owner screenshot of real report/history and confirmation of two sessions | Verified; post-upgrade phone/layout acceptance pending |
| Read/scope/revocation denials | No-cookie and publisher-only owner reads 401, wrong-project and revoked publishing 401 over private HTTPS | Verified |
| Application restart | All seven baseline reports/two session records retained; schema 3 and owner/publisher metadata unchanged | Verified |
| Data-bearing tagged upgrade | Baseline retention, compatible signed release/suite, normal automatic activation and post-upgrade reports/owner access | Pending #39 |
| Independent recovery | Encrypted off-VM copy, agreed key/policy and isolated restore | Deferred to #52 by owner |
| Normal polling | v0.3.0 ordinary scheduled poll verified; timer remained enabled/active after restart | Verified for v0.3.0; recheck after next upgrade |

Local tests and reviews remain separate from live evidence. After the remaining
#39 acceptance is actually complete, record the exact release/upgrade and owner
results, remaining limits and only then close #39 through an authorized reviewed
evidence change. Deferring recovery is not evidence that it passed.
