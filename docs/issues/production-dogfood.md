# Production agent progress dogfooding (#39)

Goal: prepare and validate the initial Codex-only production rollout without
expanding the milestone. GitHub stays the task tracker; agents publish short
progress and never fetch tasks or dashboard content.

Authorized now: implementation, isolated verification, independent reviews,
signed commit and draft PR. Merge, release publication, production service or
credential changes and disruptive recovery tests need their normal explicit
approval after a concrete procedure is ready. Production remains v0.2.2.

Plan:
1. Align README, architecture and agent guidance with implemented publish-only
   observation; distinguish local foundations from live acceptance.
2. Add opt-in systemd persistent state configuration outside release trees,
   preserving application/updater separation.
3. Add private consistent SQLite backup and new-destination restore commands,
   with validation, no overwrite and revocation of restored credentials/sessions.
   Preserve owner password/history; issue fresh publisher tokens after restore.
4. Exercise restart, release-directory replacement and backup/restore locally.
5. Write exact phased release/rollout/recovery procedure with no secrets in chat
   or logs, explicit owner phone steps and live evidence still pending.
6. Full quality gates, ARM64, sequential independent reviews, draft PR. Keep
   issue open until actual production acceptance has been recorded.

No schema/dependency/version change is intended. Release version, private setup
link delivery and production changes remain decisions after preparation. Backup
schedule/retention/off-machine destination are not silently selected.

Preparation implemented: opt-in state drop-in, private atomic snapshots and
new-path restore with credential invalidation, aligned canon and phased rollout.
Seven snapshot/recovery tests and one real process persistence test pass. Initial
Rust coverage 96.89% overall; local Clippy/canon/diff and isolated unit verification
pass. Read-only pinned production preflight confirms the unchanged baseline.
Full final validation/ARM64 and sequential reviews are required before delivery;
their final results belong in the PR, separately from live acceptance.
Live release/configuration, private delivery/backup decisions and all owner/real
Codex production acceptance remain unfinished; retain this working document.

The first security review found stale destination journals could undo restored
revocations. A new refusal regression failed first, then passed after companion
checks before copying and publication. The complete recovery suite passes.
