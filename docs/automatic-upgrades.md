# Automatic upgrades and recovery

Issue [#6](https://github.com/breki/silkboard/issues/6). Implementation is
under development. The production guest still runs the initial `v0.1.1`;
automatic upgrade, rollback, retry, and updater reboot evidence are pending.

## Release admission

The root-owned installed updater polls every ten minutes. It considers stable
`vMAJOR.MINOR.PATCH` tags in descending numeric order, accepting only versions
newer than the installed release. Prereleases, leading-zero aliases, untagged
commits, and automatic downgrades are excluded. Tags awaiting checks are
reconsidered on subsequent polls; a higher pending tag does not prevent a
lower eligible newer release from deploying.

Before building, the updater requires an annotated SSH-signed tag whose
embedded name matches its ref and whose direct target is the exact commit.
Git verifies against the independently installed
`/etc/silkboard/release-signers`, not a trust file supplied by the candidate.

GitHub evidence must identify repository `breki/silkboard` (ID `1417054470`),
active workflow `387356305` at `.github/workflows/release.yml`, a tag-push
event for the exact tag and commit, and successful completion. The latest
run's reusable CI reference must identify `refs/tags/TAG` and the same commit;
a same-named branch push is insufficient evidence. The latest
matching run must succeed; a newer failed or pending run revokes an older
success. All three jobs must succeed in that run's current attempt:

- Signed stable tag and exact revision.
- `gates / Full validation (Linux)`.
- Build the checked release revision.

The updater checks the run again after reading its jobs and repeats admission
after building. A tag deleted or moved during the build cannot activate the
original candidate. API/network failures stop the poll and preserve the
running release; they do not count as successful checks.

## Accounts and installed code

The owner selected a fine-grained GitHub token restricted to this repository,
with Contents/read and Actions/read. Store the token only in root-owned mode
600 `/etc/silkboard/updater-token`. Choose its expiry in GitHub and renew it
before that date; an expired/revoked token stops updates while the application
continues running. No existing broad owner token is copied into the guest.

The root wrapper reads the token from the fixed protected file. A private runtime copy
is readable only by the separate `silkfetch` downloader account, removed at
command completion. The token is never passed in process arguments or URLs.
Recovery and state inspection still work when the credential file is absent;
only network admission needs it.
The builder and application cannot read it, its fetch repository, or the
updater's persistent state. A root-owned source bundle readable by the builder
is the only handoff from download to build. The downloader reads that bundle
with its own permissions before root writes the snapshot, preventing a
downloader-controlled symlink from making root disclose private files.

`silkbuild` compiles the exact verified source with the existing pinned
toolchains, sequentially inside the guest. The root updater executes only
operator-installed binaries/helpers, never candidate scripts with privileges.
The application runs as `silkboard` under its existing sandbox. Updater
installation and future changes to its privileged code remain operator
actions; an application release does not replace the updater automatically.

## Installation and first admission check

Complete the [VM bootstrap](../deploy/vm/README.md) first. Build the updater
from an operator-reviewed commit using Rust 1.93.1 on ARM64:

```sh
cargo build --locked --release -p silkboard-updater
```

If using the guest's unprivileged builder, transfer a reviewed source bundle
into a root-owned directory readable by `silkbuild`, clone it with
`--no-local --no-checkout`, and check out the exact reviewed commit before
the build. Use `CARGO_HOME=/var/lib/silkboard-build/cargo`,
`RUSTUP_HOME=/var/lib/silkboard-build/rustup`, and `CARGO_BUILD_JOBS=1`.
Snapshot the resulting binary through `runuser -u silkbuild -- cat ...`
into a root-owned file; do not let root follow a builder-controlled source
symlink. Record the source revision and binary checksum for installation.

Copy `deploy/vm/` scripts and units together into a root-owned directory,
and transfer the token securely over the existing private SSH channel into
a separate root-owned mode 600 file. Keep secrets out of source bundles,
command arguments, logs, and chat. On the guest, as root:

```sh
bash /root/silkboard-updater/install-updater.sh \
  /root/silkboard-updater/updater /root/silkboard-updater-token
silkboard-update inspect v0.1.1
silkboard-update status
```

The installer creates `silkfetch`, installs fixed helpers and units, and
reloads systemd. It leaves the timer's enabled/running state unchanged.
Initial installation does not activate an application. After inspecting the
known eligible release, enable the agreed polling when authorized:

```sh
systemctl enable --now silkboard-update.timer
systemctl start silkboard-update.service
systemctl list-timers silkboard-update.timer
journalctl -u silkboard-update.service
```

The timer starts one minute after boot, catches up a missed scheduled run,
and polls on each ten-minute clock boundary. A single lock serializes timer
and operator commands. An update
may take longer than ten minutes; systemd does not run duplicate instances.
Network requests are bounded; the complete update has a 30-minute timeout.

## Failure, interruption, and manual recovery

Before building, the updater atomically writes and syncs
`/var/lib/silkboard-updater/state.json`, recording the previous and candidate
identities and blocking that candidate tag. Builds and staging leave the
current release intact. Root snapshots regular artifacts only, validates the
snapshot's identity, removes set-ID bits, and syncs staging before activation.
Activation atomically replaces `/opt/silkboard/current`, restarts the service,
and checks both health endpoints plus exact version/tag/commit identity.

Only successful health checks clear the transaction and candidate block.
A failed build, restart, or health check restores the previous release and
checks its health. A killed updater or guest reboot leaves a durable pending
transaction: the next poll restores the previous release before fetching or
building anything else. If recovery fails, the transaction stays pending for
the next run and the journal reports the error. The failed tag stays blocked,
while newer eligible releases may deploy after recovery succeeds.

Inspect state and logs before retrying. After correcting the cause:

```sh
sudo silkboard-update status
sudo journalctl -u silkboard-update.service -n 150
sudo silkboard-update retry v0.2.0
sudo systemctl start silkboard-update.service
```

Retry only clears the named block; it does not bypass verification, force a
downgrade, or change the current release. If recovery is still pending,
complete a poll first. Preserve state and release directories for diagnosis.
Current, prior releases, and build trees are retained; automatic pruning is
not implemented. Monitor disk space and remove only operator-confirmed unused
build/release directories. Disk exhaustion can prevent recovery writes;
free space before starting another recovery attempt.

To suspend polling, stop/disable `silkboard-update.timer`. An already running
service continues; stopping it interrupts the transaction, which must recover
on the next poll. Manual downgrade/pinning is not provided by this updater.
Do not edit the current symlink to a candidate awaiting verification.

## Verification status

Local behavioral tests cover numeric selection, pending check reconsideration,
workflow/run/job binding, failed build/start/health, revoked eligibility,
manual retry, interrupted recovery, private persistent state, actual symlink
switching, artifact snapshots, and secret transport without a real token.
These tests do not establish a working production updater. Live automatic
upgrade, rollback/retry, account separation, and guest reboot demonstrations
remain required before issue #6 or milestone 1 is complete.
