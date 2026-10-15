# web-dev

Read [architecture guidance](architect.md), [quality](../quality.md), and
[tooling status](../tooling.md). The web stack is selected but not installed;
inspect actual manifests and configuration before naming commands or ports. A
request to import the application foundation is separate from using this
workflow on existing code.

Keep handlers thin, test route composition and authorization, and serve the
frontend/API from the same origin. Use Svelte 5 patterns and the actual Vite
configuration. Derive version information from the configured Cargo version
source and exact release revision; avoid independent hardcoded copies.

Test meaningful UI behavior with isolated browser-test ports and no reuse of
unrelated dev servers. Never kill a listener just because it occupies a common
port. Use distinct port blocks for concurrent projects/worktrees. Verify
loading, errors and relevant API responses. Do not substitute type checking
for a browser behavior claim. Authentication and deployment-policy choices
follow agreed requirements, not assumptions copied from template examples.
