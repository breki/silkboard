# Local application development

The minimal application adapts rustbase's Axum backend and Svelte/Vite
frontend. It serves a status page, `/health`, and `/api/status`. It has no
project data, execution API, VM access, or updater credentials. Production
provisioning and automatic upgrades remain separate work.

## Prerequisites and first run

Use Rust with edition 2024 support, rustfmt, and Clippy, plus Node 22.12+
(or 24+) and npm. The optional [mise](https://mise.jdx.dev/) configuration
pins Node 24.14.1; `mise exec node@24.14.1 -- <command>` also works without
changing global settings. Run the following from the repository root:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
cargo run --locked -p silkboard-web
```

Open <http://127.0.0.1:3000>. `/health` reports process liveness as
`{"status":"ok"}`. `/api/status` reports health, package version, release
tag, and commit. Health describes a successful application response; it
makes no claim about deployment eligibility or external services. The page
fetches status once on load, with a ten-second timeout; reload to refresh.

By default development builds show `unknown` for tag and commit. Identity
comes from the backend binary, never from an independent frontend version.
The process needs only its binary and the built `frontend/dist` directory;
it does not inspect Git, contact GitHub, or run commands.

`SILKBOARD_LISTEN` accepts a socket address (default `127.0.0.1:3000`).
`SILKBOARD_FRONTEND` selects the built public assets directory (default
`frontend/dist`, relative to the process's working directory). Keep it
limited to public assets. Startup fails if `index.html` is missing.
The server handles Ctrl+C and, on Unix, SIGTERM for graceful shutdown.

For frontend hot reload, keep the backend running and in another terminal:

```sh
npm run dev --prefix frontend
```

Open <http://127.0.0.1:5173>. Vite proxies `/api` and `/health` to the
backend on loopback port 3000; keep the default backend address for this
setup. Both listeners default to loopback; private production HTTPS is
still pending.

## Release build identity

Release builds require both `SILKBOARD_RELEASE_TAG` (exactly `v` followed by
the crate version) and `SILKBOARD_COMMIT` (the full 40-character hexadecimal
Git commit). Partial or malformed identity fails development builds too.
The build script tracks environment changes so Cargo rebuilds the identity.
For an already checked-out release revision, for example:

```sh
npm ci --prefix frontend
npm run build --prefix frontend
SILKBOARD_RELEASE_TAG=v0.1.0 SILKBOARD_COMMIT="$(git rev-parse HEAD)" \
  cargo build --locked --release -p silkboard-web
./target/release/silkboard-web
```

The version must match `crates/silkboard-web/Cargo.toml`; `v0.1.0` is the
initial version. These inputs label a build; they do not verify the checkout,
signatures, clean worktree, or GitHub checks. A future release builder/updater
must establish those facts and supply the exact approved identity before
building. This command alone does not produce a deployment-eligible release.

## Checks

```sh
cargo fmt --all -- --check
cargo xtask clippy
cargo xtask doc
cargo xtask test
npm run check:all --prefix frontend
npm audit --prefix frontend
cargo audit --deny warnings
```

The Rust tests cover API responses, rejected mutation routes, static asset
serving, build identity validation, and existing repository tooling. Frontend
component tests cover loading, identity, and failed requests. The frontend
check runs types, lint, formatting, tests, and the production build.
See [tooling status](ai-agents/tooling.md) for unavailable aggregate/release
gates. These commands do not establish a Pi deployment.

The frontend lockfile retains upstream versions where possible, with security
patches selected after advisory checks. Vitest is pinned to 4.1.11, and
`nanoid`, `postcss`, and `undici` overrides retain patched versions that met
the 14-day cooldown on 2026-10-17. Reassess these pins during dependency
updates; do not blindly refresh to newer patches. Both lockfiles must remain
committed. npm packages are build/test dependencies; production serves only
compiled static assets.

A standalone coverage spot check can use `cargo llvm-cov --workspace
--ignore-filename-regex 'src/main.rs' --summary-only` when installed. This
measures the application library and build identity validation; it excludes
binary entry points, including xtask. The application entry point contains
listener, file, and signal I/O, checked separately by a live local smoke run.
This is not the pending workspace coverage gate.
