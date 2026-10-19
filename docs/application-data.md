# Private application data and recovery

This is implemented preparation for [#39](https://github.com/breki/silkboard/issues/39),
with private production state and owner registration enabled on v0.3.0.
Independent backups, isolated restore and live application recovery are deferred
to [#52](https://github.com/breki/silkboard/issues/52) by the owner on 2026-10-19.
They no longer block #39; no independent recovery protection is claimed.
Follow the [phased rollout](dogfood-rollout.md) for remaining persistence checks.

## Service storage

The optional [systemd drop-in](../deploy/vm/silkboard-progress.conf) sets:

- `StateDirectory=silkboard/progress` and mode 0700, owned by `silkboard`.
- `SILKBOARD_DATABASE=/var/lib/silkboard/progress/progress.sqlite`.
- The exact owner origin `https://silkboard.tailf38f2f.ts.net`.

Systemd creates the state directory before application startup and permits
writing there under the existing `ProtectSystem=strict` sandbox. Database
creation requires a private directory and creates a mode-0600 file. Journals
stay beside it; state survives service stops and release-directory changes.
Do not use `systemctl clean --what=state` or delete this directory during an
upgrade. No database is stored under `/opt/silkboard`, frontend assets or Git.

Install the reviewed drop-in as root-owned mode 0644
`/etc/systemd/system/silkboard.service.d/20-progress.conf`, then verify the unit
and reload systemd during an approved rollout. It is deliberately absent from
the bootstrap/updater installer: neither an application release nor reinstalling
the base unit silently enables data/authentication. The existing service user,
read-only releases, private home/tmp and inaccessible updater/build/download
paths remain in effect. The drop-in contains configuration, not credentials.

Startup fails on invalid/private-path/schema configuration rather than falling
back to status-only operation. A new database remains closed to publishing
until [first-owner registration](owner-setup.md) succeeds. Health/status stay
available to the updater. Owner, publisher and GitHub updater credentials are
independent; the application never receives the GitHub token.

## Consistent private snapshots

The installed compatible `silkboard-web` executable provides local operator
commands, not web backup/download routes. Set `SILKBOARD_DATABASE` explicitly.
Both input and output need existing private directories; use full paths. Source
files must be regular, non-symlink and private. Output must not exist, alias the
source or collide with a SQLite sidecar name. Existing destination companions
(the output filename followed by `-`, including journal/WAL/SHM files) cause
refusal; do not attach stale recovery files to a restored snapshot. Use a fresh
private destination directory under exclusive operator control. Schema 3,
SQLite quick-check and
foreign-key consistency are checked; unknown/older schemas are refused without
migration. Use trusted Silkboard snapshots, not arbitrary SQLite imports.

For a running configured service, first create a private export directory,
then take a uniquely named snapshot as the application account:

```sh
sudo -n -u silkboard install -d -m 0700 /var/lib/silkboard/progress/exports
sudo -n -u silkboard env \
  SILKBOARD_DATABASE=/var/lib/silkboard/progress/progress.sqlite \
  /opt/silkboard/current/silkboard-web database backup \
  /var/lib/silkboard/progress/exports/SNAPSHOT.sqlite
```

Choose a new `SNAPSHOT` name each time. Backup reads the live database without
migrating it and uses `VACUUM INTO` for a consistent committed snapshot,
including WAL data. It preserves owner passwords, browser/publisher hashes,
revocation and setup state as of that snapshot; the live database is unchanged.
An incomplete temporary snapshot is never published to the requested filename.
The command syncs the finished file and publishes it without replacing an
existing path. Concurrent exports to one path yield one winner. Successful
commands print a fixed completion message on stderr and no data on stdout.
Failures return nonzero with fixed errors; do not treat them as successful copies.
A crash may leave a private `.silkboard-snapshot-*` temporary file; it is not a
completed backup and can be removed after confirming no export is active.

A local export is readable/modifiable by the application account and shares its
failure domain. It is not independent recovery protection. Pull it through the
approved pinned SSH route into a new private operator file, capturing stdout
directly to the file rather than tool/chat/session output. Protect it with the
owner's chosen encrypted backup storage and test that copy. Never print/query
password hashes, sessions, progress contents or credentials in reports. Keep
backups outside the checkout, static assets and release/build source bundles.

No recurring backup service, frequency, retention or off-machine destination
is selected here. Agree those operational choices before treating backups as
ongoing protection. Take and verify a fresh independent snapshot before any
future schema-changing release, recovery or destructive operation.

## Restore into isolation

Restore always creates a **new** private database. It never overwrites a running
or existing path. On an isolated operator machine/directory, using the reviewed
compatible binary and a private copy of the snapshot:

```sh
install -d -m 0700 /PRIVATE/restore-rehearsal
SILKBOARD_DATABASE=/PRIVATE/restore-rehearsal/progress.sqlite \
  /REVIEWED/silkboard-web database restore /PRIVATE/SNAPSHOT.sqlite
```

Restore validates the source, copies it consistently and atomically publishes
the complete result as mode 0600. It retains messages, sessions and the owner
password hash, but revokes **every publisher token**, deletes browser sessions
and clears pending setup links. Existing owner setup stays permanently closed;
a never-initialized owner can receive a fresh setup link. Source backups are
unchanged. Restored installation labels remain reserved: use new labels and
privately install new publisher tokens. The owner signs in with the existing
password, or the operator uses the private [password reset](owner-login.md#operator-setup-and-reset)
pipe. Revocation after the snapshot must never silently become valid again.

For a rehearsal, do not expose a listener to production clients or run Codex
hooks against it. Verify counts, schema, login/publisher rejection and a known
nonsecret canary, then remove the isolated copy through an approved cleanup.
Never put its credential hashes or message contents in captured output.

Promoting a restore to production is a separate approved recovery operation:
pause polling and wait for the updater to finish; stop the app; preserve the
current private directory and compatible binary; restore into a fresh private
directory; set the correct `silkboard` ownership; move the old state directory
aside and put the new directory at the configured state path while stopped.
Move entire database directories, not just main files next to old WAL/journal
files. Restart the compatible release, verify exact health/permissions and
credential invalidation, then reissue tokens privately and resume polling.
Do not run these steps against production as part of an isolated test.

A symlink rollback is not a database downgrade. Older binaries may refuse a
newer schema; retain compatible backups/releases and review recovery before
publishing a schema change. Do not replace updater state or erase its failed-tag
blocks to recover application data.

## Verification boundary and references

The storage tests exercise real command processes, private modes, no overwrite,
WAL snapshots, competing exporters, invalid schemas/files/paths, retained history
and invalidated restored credentials/setup. Process tests exercise restarts and
replacement of release directories using one compiled application revision;
they do not prove a tagged production upgrade, systemd confinement or live
restore. Restart persistence is now verified live. Tagged-upgrade persistence
remains #39; independent backups and isolated restore/recovery remain #52.

Reference contracts: [SQLite VACUUM INTO](https://sqlite.org/lang_vacuum.html#vacuum_with_an_into_clause)
and [systemd execution/state directories](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html).
