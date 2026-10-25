# Production task-plan rollout

Observed on 2026-10-25 for [#81](https://github.com/breki/silkboard/issues/81).
This record separates live observations from the
[local acceptance fixtures](task-plan-acceptance.md). The owner approved the
private recovery copy, release publication, compatible publisher installation
and explicit plan-reporting opt-in. Final phone acceptance remains open.

## Signed release and activation

Production runs `v0.7.0` at
`356bdbbd24e1a77f9943a13df767f5fbefff8959`. The owner-approved SSH signer
passed local tag admission. Full local validation, ARM64 Clippy and a separate
advisory audit passed before the signed tag and main were pushed atomically.

[Release run 40169266503](https://github.com/breki/silkboard/actions/runs/40169266503)
passed on attempt 2. API read-back matched repository `breki/silkboard`,
registered workflow `387356305`, `.github/workflows/release.yml`, push event,
tag `v0.7.0` and that exact commit. Admission, full validation and the release
build all succeeded. Attempt 1 remains recorded: its phone keyboard test sent
Enter before the asynchronous breakpoint handler had switched layout. Main CI
on the same commit passed. The separately reviewed
[test synchronization fix](https://github.com/breki/silkboard/pull/94) waits
for the desktop details region to leave the DOM without changing application
code, timeouts or the existing phone assertions. The signed tag was unchanged.

After eligibility passed, the operator requested the normal installed updater
poll. This was a manually requested poll, not evidence of scheduled discovery.
The updater independently admitted the signed revision, built inside the VM,
activated it and passed health checks. Exact production version/tag/commit and
private HTTPS were verified at 07:00:58 UTC. The preceding release stayed
healthy during staging; no privileged updater or trust installation changed.

## Existing data and recovery protection

Before publication, a consistent production snapshot and the installed ARM64
executable were transferred through the existing pinned SSH administration
route into a fresh private operator directory on the development machine.
The directory was mode 0700 and data mode 0600. No database contents, password
hashes, browser credentials or real publisher tokens entered chat or Git.

A compatible local executable restored that snapshot only into isolation.
Checks verified all 169 reports, 11 sessions, projects and owner data were
retained; restored publisher authority and browser sessions were revoked.
The source copy remained intact. No listener or live restore was used, and
the disposable restored copy was removed after verification. The private
source snapshot and recovery executables were retained. This one-off safeguard
does not settle [#52](https://github.com/breki/silkboard/issues/52)'s encryption,
destination, cadence or retention policy, or prove live recovery.

After activation, and again after real plan writes, operator metadata checks
verified:

- All prior report/session rows remained exact; 172 ordinary reports were
  present, including new reports received since the 169-report baseline.
- Owner and publisher authority were unchanged; both browser-session rows
  remained present. Owner phone access supplied separate live read evidence.
- Base schema stayed 3 and additive plan component 1 initialized correctly.
- Private directory/database modes remained 0700/0600.
- Installed updater, trust file and progress drop-in checksums were unchanged.
  The timer remained enabled/active, its earlier blocked tag was retained and
  no update transaction remained pending.

## Installed native reporting and actual work

The standalone publisher was built from the same signed release source and
installed on the existing development machine. Its SHA-256 is
`75f5cb819f131506bb32de305d8e1448b69d30f648d23b7ae05f0cb079a28058`.
The publisher package version alone does not identify its application release.
The previous executable and configuration were retained privately. Existing
endpoint/project settings and working state were preserved; only the explicit
`codex.plan_reporting` opt-in was enabled. Existing short-report hooks and
their trust were unchanged. This does not establish installation on other VMs.

The installed `codex-plan` adapter published genuine work using the native
session environment, with no invented identity or transcript reader. Exact
stored snapshot hashes and reporter identity were verified through bounded
operator metadata inspection of the submitted tasks. This inspection does not
grant the publisher owner-read or dashboard task-fetch permissions.

Two actual tasks were reported:

- `Release and dogfood task-progress graphs` records the remaining rollout.
- `Verify production task-plan reporting` began with zero completed steps,
  then reported overlapping work, a revised definition and the phone handoff.

For the second task, a temporary private configuration used an explicitly
invalid fixture credential for one request. It preserved the real endpoint,
task and working-state binding. Sequence 3 was rejected; the server remained
at sequence 2. The next normal full report, sequence 4/revision 2, repaired
the complete graph, including its added data-retention check. At that point
received history contained only sequences 1, 2 and 4 and correctly marked the
gap. Later reports retained that honest incomplete history. Temporary invalid
credential/configuration files were removed. No production service, real
credential, normal client configuration or authority was disrupted.

Private HTTPS checks confirmed unauthenticated feed and plan reads still
return 401 and first-owner setup remains closed. Repeated metadata reads kept
the same receipt time while its age increased; reads did not create freshness.

## Owner observation and remaining acceptance

The owner opened the task on their phone and explicitly confirmed received
history and the last-report time/age were visible after their locations were
explained. That answer resumed the actual phone step; subsequent reports
recorded the blocked-to-active-to-done transition. These are live reports,
not fictional mockup states or evidence that reported work authorizes actions.

The remaining owner check is the completed plan view, step-detail interaction
and the received blocked/resumed transition. Do not infer those observations
from the narrower history/time confirmation or local browser tests. Keep #81
and milestone 5 open until that final acceptance and its evidence are recorded.
Recurring backups, updater reboot recovery and other-agent integration retain
their separate scope.
