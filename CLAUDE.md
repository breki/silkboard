# Claude Code entry point

@AGENTS.md

Claude commands under `.claude/commands/` route to shared procedures.
`/architect` and `/web-dev` are skills under `.claude/skills/`.
Read the referenced procedure and resolve its links relative to that file.
Forward command arguments to it; wrappers do not redefine project policy.
Use `AskUserQuestion` for interactive choices when available, and delegate
review roles through `Agent` when supported. If a capability is unavailable,
follow the shared procedure's fallback and state the limitation.

No hooks or tool permission overrides are installed by this canon.
