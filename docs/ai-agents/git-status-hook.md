# Codex Git-status hook

The repository registers a `Stop` hook in
[hooks.json](../../.codex/hooks.json). After each completed turn it uses
[Kozmotic](https://github.com/breki/kozmotic#codex-git-status) to display branch,
ahead/behind, file counts, changed lines, and last-commit age as plain text.
The hook runs on the machine running Codex, including an SSH development VM.

## Enable

Git and Kozmotic 2.3.0 or later must be installed on that machine. Download
the appropriate package from [Kozmotic releases](https://github.com/breki/kozmotic/releases),
verify its checksums, and run the extracted binary's `self install` command.
The hook invokes `$HOME/.claude/bin/kozmotic` directly through a POSIX shell.
Windows-native setups need an appropriate command override before use.

Open `/hooks` in Codex and review/trust this repository's Stop hook. The
project `.codex` layer must also be trusted. Reload the session if the new
hook is not listed. Changed hook definitions need review again. This follows
[Codex's hook trust requirements](https://learn.chatgpt.com/docs/hooks).
The repository does not modify user-wide hooks or bypass trust checks.

If a trusted hook never runs after restarting the CLI, the CLI may have
reconnected to the same daemon and in-memory conversation. To rule out retained
session configuration, exit the CLI, restart the daemon, and resume:

```bash
codex app-server daemon restart
codex resume
```

Restarting the daemon interrupts its active sessions, so finish other work
first. A fresh hook-list check proves discovery and trust, not execution in
an existing conversation. Verify the next completed turn before claiming the
hook is operational.

A normal result looks like:

```text
Git: main | git 2mod 1new | +42/-7 | last 12m
```

Kozmotic omits widgets without a value; clean-state rendering follows the
installed Kozmotic version. Ahead/behind uses local remote-tracking refs;
the hook does not fetch. The session's `cwd` selects the Git checkout.

## Behavior and limitations

Kozmotic's native `codex-git-status` command reuses its existing Git widgets
and emits JSON containing only `systemMessage`, which Codex surfaces as a
hook message. This is a snapshot after a turn, not a live footer.
Colors and terminal controls are removed.
The summary is limited to 1,000 characters. Outside a Git worktree it stays
quiet. Invalid input or unavailable Git also produces an empty JSON object.
Kozmotic never asks Codex to continue the turn. A missing Kozmotic executable
or a timeout is handled by Codex's hook runner.

The hook does not validate code, modify Git state, or enforce release gates.
It disables Git's optional locks. The hook runner limits the command to six
seconds. There is no Python adapter or Python runtime dependency.

## Verify

Run the behavioral tests in the Kozmotic checkout:

```bash
cargo xtask test codex_git_status
```

To check the installed command manually, substitute the checkout's absolute
path in this JSON input:

```bash
printf '%s\n' '{"cwd":"/path/to/project"}' \
  | "$HOME/.claude/bin/kozmotic" codex-git-status
```

The input is JSON-encoded so paths with spaces or quotes remain valid. An
actual Codex turn after hook trust is required to verify delivery and
presentation in the user's client.
