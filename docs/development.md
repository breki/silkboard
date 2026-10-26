# Local application development

Silkboard uses a Rust/Axum backend and Svelte/Vite frontend. The private owner
pages are Agents, Events, Workflows and Todos; `/status`, `/health` and
`/api/status` expose application health and build identity. The
[standalone publisher](publishing-cli.md) builds in the same workspace.
Production provisioning and automatic upgrades remain separate workflows.

## Install and start once

Use Rust with edition 2024 support, rustfmt and Clippy. The Rust version is
pinned in the [CI workflow](../.github/workflows/ci.yml); the optional
[mise configuration](../.mise.toml) pins Node. See
[tooling prerequisites](ai-agents/tooling.md) for the complete validation
tools. From a new checkout, or after the frontend dependency manifests change:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
```

Keep that dependency installation for source-only edits. The initial frontend
build supplies `frontend/dist/index.html`, which backend startup requires.
For UI iteration, Vite serves the current source; rebuilding `dist` after each
edit is unnecessary.

In one terminal, start the backend from the repository root:

```sh
cargo run --locked -p silkboard-web
```

In another terminal, start Vite:

```sh
npm run dev --prefix frontend
```

Open <http://127.0.0.1:5173>. Use <http://127.0.0.1:5173/status> to verify the
backend connection without owner credentials. Vite serves source changes with
hot reload and proxies `/api` and `/health` to `127.0.0.1:3000`. Its configured
port is 5173 with strict binding; backend port 3000 must match that proxy.
Both listeners bind to loopback. Resolve a port conflict with the server you
started rather than connecting this setup to an unrelated listener.

Save Svelte/CSS/TypeScript edits and inspect the existing Vite page. Restart
the backend after Rust edits so the running process uses the rebuilt code.
Changes to Vite configuration or dependencies can require a Vite restart.

Use worktree-local Cargo output for the long-running backend. The optional
[local Cargo wrapper](local-build-reuse.md) holds its shared lock for a whole
command, including `run`; using it for a persistent server would make later
wrapper checks wait. The wrapper is useful for the short checks below.

The backend also serves built assets directly at <http://127.0.0.1:3000>.
That view uses `dist`, so refresh the frontend build before checking the
standalone app or running the browser fixture suite.

## Choose feedback for the edit

Run commands from the repository root. `./dev/cargo` opts into dependency
reuse between trusted worktrees; ordinary `cargo` works with worktree-local
output instead. Cargo handles source/profile/feature changes; routine edits
do not need a cache deletion. The wrapper rebuilds workspace code when its
workspace path changes, so that first command can be slower.

For compile-only feedback, check the affected crate and its test targets:

```sh
./dev/cargo check --locked -p silkboard-web --all-targets
```

For behavior feedback, run the relevant test target directly; Cargo compiles
its inputs as needed. These are independent examples:

| Changed behavior | Example focused check |
| --- | --- |
| Backend status/static HTTP behavior | `./dev/cargo test --locked -p silkboard-web --test status` |
| Publisher HTTP delivery | `./dev/cargo test --locked -p silkboard-publish --test publishing` |
| Updater policy/evidence | `./dev/cargo test --locked -p silkboard-updater --test policy_evidence` |
| Tooling CI-scope classification | `./dev/cargo test --locked -p xtask --test ci_mode` |
| Shared task-plan types and validation | `./dev/cargo test --locked -p silkboard-plan` |

For example, owner Todo changes use the backend's `todos` target. Integration target names come from the crate's `tests/*.rs` files.
A trailing test-name filter can narrow a target further. Cargo can return
success when a filter matches zero tests; check the names and executed count.
After the small check passes, run the affected crate's complete tests:

```sh
./dev/cargo test --locked -p silkboard-web
```

For frontend edits, keep Vite running and select the affected component/API
tests. This example selects the existing Header test file:

```sh
npm test --prefix frontend -- Header.test.js
```

For repeated edits, its interactive watch command is:

```sh
npm run test:watch --prefix frontend -- Header.test.js
```

Stop watch mode with Ctrl+C. Check Svelte/TypeScript consumers as well:

```sh
npm run check --prefix frontend
```

Run frontend lint and formatting checks before committing:

```sh
npm run lint --prefix frontend
npm run format:check --prefix frontend
```

For a single file while editing, use the installed tools from `frontend/`:

```sh
(cd frontend && npm exec -- eslint src/Header.svelte)
(cd frontend && npm exec -- prettier --check src/Header.svelte)
```

The full frontend check remains available as `npm run check:all --prefix
frontend`; it also runs all component tests and builds `dist`. The
[local measurements](local-feedback-measurements.md) record the scopes,
executed tests, cache conditions and observed feedback times. A focused test
result establishes only that selected behavior.

## Widen checks when the change crosses a boundary

| Change | Checks to include before the full gate |
| --- | --- |
| Shared plan models or validation | `silkboard-plan`, backend and publisher tests that consume the contract |
| Wire format, API routes, auth or persistence | Both server/client Rust tests, frontend API/component tests, and relevant browser flows |
| UI layout, interaction, focus or themes | Svelte types, affected component tests, and the built-app browser suite |
| Cargo manifests, features, profiles, lockfiles or workspace lints | Workspace checks; dependency changes also need current admission/audit checks |
| xtask dispatch or cache/build helpers | Relevant xtask integration tests and the helper's existing behavioral tests |
| Owner-handoff, hooks, VM bootstrap or deployment | Their existing handoff/Bombyx/updater suites as applicable; operational claims require their own live evidence |

For a standalone browser run, finish building the frontend first. Choose a
free eight-port block for your worktree, then run the suite:

```sh
npm run build --prefix frontend
SILKBOARD_E2E_PORT_BASE=18164 ./dev/cargo xtask browser-test
```

The fixtures serve built assets. Keep `dist` stable while that run executes;
a concurrent rebuild can replace files being requested by the browser. The
suite uses disposable local storage and credentials, not production data.
See [tooling](ai-agents/tooling.md) for Chromium/OpenSSL prerequisites and
[feed verification](owner-feed.md#verification) for its coverage.

Code changes still require the complete read-only validation suite before
merging:

```sh
./dev/cargo xtask validate --check --parallel
```

The bounded parallel schedule, coverage/security thresholds and full browser
checks remain unchanged. Documentation-only changes use the documented canon
and diff checks under [quality rules](ai-agents/quality.md). Release work also
requires its separate ARM64 lint, advisory audit and signed exact-revision
checks in the [release procedure](ai-agents/workflows/release.md). Local checks
do not establish hosted release-suite success or production activation.

## Backend settings and owner access

Development binaries show `unknown` tag/commit unless build identity is
supplied. Identity comes from the backend rather than a frontend version.
The process serves its binary and public assets; it does not inspect Git or
execute commands. Health describes an application response, not deployment
eligibility or the health of external services.

`SILKBOARD_LISTEN` selects the listener (default `127.0.0.1:3000`).
`SILKBOARD_FRONTEND` selects the built public assets directory (default
`frontend/dist`, relative to the process's working directory). Keep that
path limited to public assets. Ctrl+C or Unix SIGTERM stops the server.

The default setup has no database or owner account. `SILKBOARD_DATABASE`
enables [persistent ingestion](progress-ingestion.md);
`SILKBOARD_OWNER_ORIGIN` additionally enables
[owner authentication](owner-login.md) and the [private feed](owner-feed.md).
Owner login needs an HTTPS proxy for its Secure cookie. The HTTP Vite server
alone is not an authenticated owner-testing setup; use the existing local
HTTPS browser fixtures or the documented owner setup. The
[production VM](production-vm.md) uses private Tailscale HTTPS.

## Release build identity

Release builds require both `SILKBOARD_RELEASE_TAG` (exactly `v` followed by
the crate version) and `SILKBOARD_COMMIT` (the full 40-character hexadecimal
Git commit). Partial or malformed identity fails development builds too.
The build script tracks these environment changes. For an already verified
release checkout:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
SILKBOARD_RELEASE_TAG="$(git describe --tags --exact-match HEAD)" \
  SILKBOARD_COMMIT="$(git rev-parse HEAD)" \
  cargo build --locked --release -p silkboard-web
./target/release/silkboard-web
```

The tag version must match the workspace package version in `Cargo.toml`.
These inputs label the binary. The release workflow and updater separately
verify signatures, checkout identity, cleanliness and GitHub evidence before
admitting a revision. See [release preparation](releases.md).
