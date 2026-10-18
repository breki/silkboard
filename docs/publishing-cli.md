# Standalone progress publisher

`silkboard-publish` implements [#36](https://github.com/breki/silkboard/issues/36)
as an installed Rust executable. Target machines need no Rust, Cargo, Node,
curl, SQLite service or background process. This implementation supports
Linux GNU on x86-64 and ARM64, with a compatible system runtime. HTTP/TLS
dependencies are compiled in. There is no outgoing queue or automatic retry;
the owner accepts lost messages during outages for this milestone.

## Build and install

On a trusted build machine with the pinned Rust toolchain and a C compiler:

```sh
cargo build --locked --release -p silkboard-publish
```

Copy `target/release/silkboard-publish` to a machine with the same architecture
and a compatible Linux runtime. The source checkout/build caches are not
needed there. On that target:

```sh
install -d -m 0755 "$HOME/.local/bin"
install -m 0755 /path/to/silkboard-publish "$HOME/.local/bin/silkboard-publish"
"$HOME/.local/bin/silkboard-publish" --version
```

Ensure that directory is on `PATH`. Third-party attribution texts are embedded
and available with `--licenses`; preserve/refresh them when dependencies change.
Upgrade by replacing the executable with another reviewed compatible binary.
No service or self-updater is installed. This implementation publishes no
downloadable release binary. Distribution and production installation still
follow the normal reviewed release process; a local build is not a release
or evidence of ARM64 execution.

## Private configuration

```sh
install -d -m 0700 "$HOME/.config/silkboard-publish"
install -d -m 0700 "$HOME/.local/state/silkboard-publish"
```

Install the project token privately as a mode-0600 file in the configuration
directory. The current [operator command](progress-ingestion.md#local-setup-and-credentials)
issues it without printing the secret. Web token management is separate
[#44](https://github.com/breki/silkboard/issues/44); first-owner setup is
[#45](https://github.com/breki/silkboard/issues/45). Never paste tokens into
chat, command arguments, URLs or reports.

Save `config.json` with mode 0600 in that directory, replacing the sample
paths with absolute paths on this machine:

```json
{
  "endpoint": "https://silkboard.tailf38f2f.ts.net",
  "project": "silkboard",
  "token_file": "/home/USER/.config/silkboard-publish/publisher.token",
  "state_dir": "/home/USER/.local/state/silkboard-publish"
}
```

The endpoint is a lowercase HTTPS origin without credentials, trailing slash,
path, query or fragment. Literal `http://127.0.0.1:PORT` and
`http://[::1]:PORT` are allowed for local development. TLS verifies the host
and bundled public roots. Redirects/environment proxies are disabled; there
is no insecure-TLS option. Network attempts have a three-second budget,
bounded headers and a 1,024-byte acknowledgement limit.

Configuration/token files must be regular mode-0600 files without hardlinks,
in mode-0700 immediate directories. Their paths and the state directory must
be absolute and free of symlink directory aliases. The operator controls
the directories and ancestors. Unknown fields and oversized/invalid input
are rejected; configuration is limited to 16 KiB and reports to 8 KiB.

## Publish

```sh
silkboard-publish publish --config "$HOME/.config/silkboard-publish/config.json" <<'JSON'
{"hostname":"development-vm","tool":"codex","session_id":"native-session-id","message":"Running tests"}
JSON
```

Optional fields are `subagent_id`, `parent_session_id` and `session_name`.
Use actual native identities, with separate child identity for subagents;
names never replace identity. Parent metadata must remain consistent. The
[ingestion contract](progress-ingestion.md#http-contract) defines field limits.
Publish short observations, not transcripts, handoffs, files or command logs.

Exit 0 with `Progress acknowledged` means the exact event ID was acknowledged.
Exit 1 reports a fixed input/storage/credential/network/response diagnostic
without tokens, messages or raw server errors. An unsuccessful attempt may
have reached the server before its reply was lost. Running the command again
creates a new event, rather than replaying the old one. No report is saved
for later delivery. Responses supply acknowledgements only, never tasks,
instructions or a new endpoint.

## Counters and integration

The private state directory contains hashed-identity counter/lock files per
endpoint/project/host/tool/native-session/child key. Each counter stores only
its schema, last allocated sequence and optional parent ID. It stores no
report or credential. Allocation uses a nonblocking file lock, atomic replace
and filesystem synchronization before sending. Busy callers return promptly.
Failed attempts consume sequences; gaps/out-of-order arrival are valid.

Keep counters when upgrading or resuming sessions. Use local filesystems and
do not replace counter/lock files during use. Do not restore old counters
against newer server history. Corrupt/newer-schema state, changed parent
metadata and exhausted counters fail closed. Repair permissions/storage;
do not delete active counters to clear errors. Lost state requires a fresh
native identity or operator reconciliation; the client never reads history.

The [Codex adapter](codex-publishing.md) provides opt-in automatic invocation
through native async hooks, tolerates publishing failure and preserves
validation hooks and development permissions. Claude is separate
[#43](https://github.com/breki/silkboard/issues/43). Production setup, credentials,
rollout and phone verification remain [#39](https://github.com/breki/silkboard/issues/39).

`cargo test -p silkboard-publish` checks real HTTP/ingestion, process restarts,
concurrency, revocation, malformed responses and private files.
