# Silkboard development in a Bombyx VM

This is native Ubuntu 24.04 guest provisioning for
[#13](https://github.com/breki/silkboard/issues/13), without containers. Bombyx
clones `main` into `/home/agent/silkboard` and runs `.bombyx/provision.sh` as
`agent`, with passwordless guest sudo. These files must be merged to `main`
before the operator creates the VM. They do not create a VM, open network
access, install production trust or authenticate either AI service.

## Provision and enter

Use the operator's existing external Bombyx project configuration and KeePassXC
vault. Keep secrets and VM sizing outside this repository. The source repository
is `git@github.com:breki/silkboard.git`, project `silkboard`, ref `main`, and
provisioning script `.bombyx/provision.sh`. Set the project's `secrets_refreshed`
hook to `.bombyx/refresh-secrets.sh`. The vault supplies the deploy-key attachment
and `SILKBOARD_PUBLISH_TOKEN`; do not copy vault contents into this checkout.

The operator reviews CPU/RAM/disk choices, checks `bombyx list --offline`, then
runs `bombyx up silkboard`. Use `bombyx shell silkboard` to enter. To rerun the
bootstrap on an existing VM use `bombyx provision silkboard`; another `up` does
not rerun provisioning. Follow the installed Bombyx help for its configuration
path; this repository does not invent a second registry.

Provisioning installs system build/browser/cross-compiler prerequisites and:

- Rust 1.93.1, rustfmt, Clippy, llvm-tools-preview and ARM64 target support.
- Node 24.14.1 and gate tools pinned by the existing CI/tooling guide.
- Codex through OpenAI's standalone installer, independently of Node/npm.
  Fresh installations and migration select the current stable release;
  repeated provisioning preserves the installed standalone version.
- Codex's persistent user defaults in `~/.codex/config.toml` select
  **Approve for me**: `approval_policy = "on-request"`,
  `approvals_reviewer = "auto_review"` and `default_permissions = ":workspace"`.
  Reruns restore these defaults through Codex's atomic configuration API,
  preserving unrelated settings. Explicit session/profile settings and managed
  organization requirements can still override the user defaults.
- A standing, publish-only Silkboard progress authorization in Codex's
  `[auto_review].extra_policy`. It covers the installed publisher and its
  existing owner-configured destination/credentials, including necessary local
  reporting-state writes. It does not authorize dashboard reads, deployment,
  destination changes or unrelated requests. Provisioning preserves other
  owner policy text, updates only its marked block, and refuses a concurrent
  configuration change instead of overwriting it. Managed policy still takes
  precedence; this is reviewer guidance, not a blanket approval guarantee.
  Existing boolean `codex.plan_reporting` opt-ins in the private publisher
  configuration are retained during reprovisioning; a fresh configuration
  still requires the operator to enable reporting explicitly.
- Kozmotic 2.3.0 at `~/.claude/bin/kozmotic`, required by the repository's
  Git-status hook. Fixed release archive and binary checksums cover x86_64 and
  ARM64; reruns retain the exact installed binary and refuse conflicting files.
- Claude Code 2.1.293, published 2026-10-07 and older than fourteen days when
  selected on 2026-10-22. Installing Claude does not enable Claude reporting;
  that integration remains #43.
- GitHub CLI (`gh`) from GitHub's signed package repository, avoiding Ubuntu's
  older community package with deprecated API behavior.
- Tailscale from its signed stable Ubuntu 24.04 repository, with `tailscaled`
  enabled. Provisioning does not log in, advertise routes, enable Tailscale SSH
  or change the VM host's firewall. Repeated provisioning retains its guest
  device identity and authentication state.
- Locked frontend dependencies/Chromium and a release-built standalone publisher.

Node/rustup archives use fixed official SHA256 values for x86_64 and ARM64.
Claude's npm package uses a user-owned prefix, never `sudo npm install`. Repeating
provisioning retains compatible installed tools, configuration and counters;
it refuses conflicting publisher configuration and unrelated command files.
No personal agent configuration, authentication cache or hook trust is copied.
Run provisioning while agent sessions are idle, since rebuilding/installing
programs can replace the executables they use.

### Codex installation and upgrades

The bootstrap calls `.bombyx/install-codex.sh`, which uses system Bash, curl
and the official standalone installer without invoking npm. Codex's default
launcher is `~/.local/bin/codex`, pointing through
`~/.codex/packages/standalone/current` to the selected release. The installer
owns that layout and its normal update flow; provisioning does not pin or
restore an older Codex version. Claude installation/repair is independent.
Initial selection of the current stable release is intentional for this
user-updated development tool; the former 0.162.1 hook-verification pin no
longer controls installation. This does not change locked application
dependency cooldown or release gates.

On an existing VM, provisioning recognizes the previous launcher into
`~/.local/share/silkboard-tools/agents/bin/codex`. After the standalone
installer succeeds and its version is verified, the helper retires recognized
npm Codex launcher symlinks in both that prefix and the provisioned Node
runtime. The latter can contain a second copy from an ineffective npm update.
Old npm package files remain inert; no npm uninstall scripts run. Codex
authentication, configuration and sessions are retained in `~/.codex`.
Unrelated command files/symlinks, a broken standalone selection or a custom
`CODEX_HOME` are refused for explicit inspection. Failure before cutover
leaves the old launcher available; rerunning can retry the installation.

With agent sessions idle, apply the merged provisioning changes using the
existing Bombyx provisioning flow. For a narrow Codex-only migration in an
already updated guest checkout, run:

```bash
BOMBYX_PROJECT=silkboard bash .bombyx/install-codex.sh
hash -r
command -v codex
readlink -f "$(command -v codex)"
codex --version
```

After migration, accept Codex's ordinary supported update prompt; no custom
npm-prefix command is needed. The official installer can also be rerun as
documented by OpenAI. Open a new shell or clear its command cache with
`hash -r` after migration. Verify the resolved path is under the standalone
release tree and compare `codex --version` with
`~/.local/bin/codex --version` after updating. Reprovisioning must retain that
updated version.

Behavioral fixtures cover migration, private-state preservation, install
failure/retry, conflicting commands, PATH resolution, repeat provisioning and
independent Claude repair. On 2026-11-08 a disposable local home also verified
the real official installer: fresh 0.167.0 installation, a standalone
0.163.0-to-0.167.0 upgrade, resolved release path/version agreement and helper
rerun preserving 0.167.0. The real `codex update` command also successfully
invoked the standalone installer for a same-version update. Codex emitted a
warning about creating its auxiliary
PATH aliases under `/tmp`; the visible standalone command and installer still
succeeded. This is not live guest migration, update-prompt interaction or
native reporting evidence; verify those separately before claiming the VM
migration works.

## Commit author and signing

The owner selected `Silkboard VM <silkboard-vm@agents.invalid>`. The `.invalid`
address has no mailbox and does not attribute commits to a GitHub account.
Provisioning sets this identity and SSH commit/tag signing only in the clone's
local Git configuration. It preserves Bombyx's `core.sshCommand` and signs with
`~/.ssh/bombyx-deploy-key`. It never transfers the owner's personal key.

The separate VM key has both repository write and release-signing authority
once independently installed production trust permits it. Guest code can read
that key; the VM's passwordless sudo does not isolate it from its agents. See
[release trust and revocation](releases.md#vm-signer-and-revocation) before
installing production trust. A GitHub Verified badge is not the trust decision.

## Private publisher and refresh

The project and commit author are in `.bombyx/settings.json`. Supply the real
endpoint through your external Bombyx registry; there is no endpoint default:

```toml
[projects.silkboard.env]
SILKBOARD_PUBLISH_ENDPOINT = "https://silkboard.example.test"
```

Replace this reserved example origin with your own lowercase HTTPS origin,
without a trailing slash, path, query, credentials or fragment. Missing or
invalid input fails configuration; it never selects another deployment. Keep
your real address outside Git. Existing differing publisher configuration is
refused rather than silently redirected. Provisioning creates:

- `~/.local/bin/silkboard-publish` and user-visible tool links.
- `~/.config/silkboard-publish/config.json`, mode 0600, with the actual checkout,
  guest hostname and optional Codex reporting configuration.
- `~/.config/silkboard-publish/publisher.token`, mode 0600.
- `~/.local/state/silkboard-publish/`, mode 0700; counters are retained on reruns.

Configuration and token directories are private. The refresh hook uses the
installed Node runtime by absolute path, so Bombyx's bare environment works.
It parses `$BOMBYX_ENV_FILE` as data, never sources shell code, accepts the
64-lowercase-hex publisher token and atomically replaces the destination.
It supports CRLF, `export`, quoting, comments and last-assignment-wins for the
one selected variable. As specified by Bombyx, a quoted value ends at its
closing quote; trailing bytes are ignored, not executed. An unterminated quote
is refused. Secret input is capped at 64 KiB and must be a regular,
private owner-controlled file. Unsafe paths, symlinks and hardlinks are refused.
Missing, malformed or removed input returns failure and removes any safely
identified old destination token, preventing stale publishing after removal.
An unsafe existing destination is left untouched for operator inspection.
No token, raw input or exception details are printed, including on failure.

Bombyx rewrites its private `~/.bombyx-env` copy and runs the configured refresh
hook on provisioning and `bombyx shell silkboard --refresh-secrets`. The hook
is safe to run twice and needs no login shell or input. Bombyx does not run the
hook for scratch VMs. Rotation does not reset publisher counters or change the
server's credential policy. Revoke retired credentials using the owner UI.

## Authentication and live verification

Provisioning installs programs, not authenticated accounts. Inside the guest:

1. Confirm `rustc --version`, `node --version`, `codex --version`,
   `claude --version`, `gh --version`, `tailscale version` and
   `silkboard-publish --version`.
2. Use `codex login --device-auth` and follow its private browser instructions.
   Start Claude and complete its own `/login` flow. Do not paste credentials,
   one-time codes or login links into repository records or chat.
3. Review the repository's Codex handlers in native `/hooks` before enabling
   reporting. Preserve validation/Git-status hooks; provisioning selects
   **Approve for me** defaults without enabling bypass flags or editing trust
   records.
   Provisioning installs the informational Git-status handler's Kozmotic
   prerequisite; it does not copy the workstation's binary. Follow the
   [Git-status setup guide](ai-agents/git-status-hook.md#enable) to verify the
   next actual turn. This informational hook does not replace release gates.
4. Register this guest as its own private tailnet device: run
   `sudo tailscale up --hostname=silkboard-dev` in the guest and approve the
   login on your phone. Confirm `tailscale status` reports it connected before
   checking the endpoint. No auth key, login URL or tailnet address belongs in
   the repository. Tailscale account approval remains an operator action.
   Check `curl --fail "$SILKBOARD_PUBLISH_ENDPOINT/api/status"` during
   provisioning, or use the endpoint from your private publisher config after
   entering the guest (Bombyx provisioner environment variables do not persist
   in interactive shells). The private
   endpoint needs an approved guest route/DNS/Tailscale arrangement. The VM host's
   guest-to-host filtering must not be assumed to provide access. Choose/fix
   connectivity explicitly rather than weakening the publisher's HTTPS policy.
5. Run `cargo xtask validate --check` and ARM64 Clippy from the guest. Complete
   actual authenticated work in each AI tool and verify ordinary Codex reports
   appear with the guest hostname/native identity in the owner's phone feed.

## GitHub CLI credential refresh

Installing `gh` alone does not provide GitHub API access. The operator creates
a fine-grained token limited to `breki/silkboard`, with Contents, Pull requests
and Issues read/write and Actions read. Store it in the external vault entry
`GitHub/silkboard gh token` and map `vault.entries.GH_TOKEN` to that entry in
the external Bombyx project configuration. Token creation, vault changes and
that external mapping are operator steps; no credential belongs in this repo.

Bombyx writes `GH_TOKEN` beside the publisher secret in its private env file.
The same refresh hook reads each selected key as data, never sources the file,
and refreshes both credentials independently. A bad publisher token still
removes its old token file and does not block GitHub login; a GitHub failure
does not skip or delete a valid publisher token. Each failed operation reports
a fixed message without input or subprocess output, and either failure makes
the hook exit nonzero.

The hook accepts a `github_pat_` token and sends it only on stdin to
`/usr/bin/gh auth login --hostname github.com --with-token --insecure-storage`.
The child has a minimal environment without credential variables or debug
settings. Login checks the token online. The headless guest stores it in
`~/.config/gh/hosts.yml`; the hook requires private, owner-controlled storage,
with mode 700 for the `gh` directory and 600 for existing config files. No
workstation `gh` authentication/configuration is copied.

Repeated login and successful token rotation replace the active credential.
Failed login can leave the previously stored credential in place. Removing
the `GH_TOKEN` line explicitly removes all cached `github.com` accounts using
offline account lookup and logout; an already empty store succeeds. Other
GitHub hosts are unchanged. Removal does not revoke the token at GitHub.
All `gh` calls share a twenty-second deadline, inside Bombyx's sixty-second
hook budget; failures and timeouts are reported without claiming success.

Secret refresh runs the hook already in the guest checkout; it does not fetch
new code. After merge, first use `bombyx shell silkboard` without refresh. With
agent sessions idle and a clean `git status --short`, update the guest clone:

```bash
cd ~/silkboard
git fetch origin main
git merge --ff-only FETCH_HEAD
```

If the checkout has changes or the fast-forward fails, stop and preserve that
work before proceeding. Exit the guest shell. After the operator supplies the
vault mapping, run `bombyx shell silkboard --refresh-secrets` on the operator
machine, then in the guest run `gh auth status --hostname github.com`. Do not
use `--show-token` or share the credential files. A fixture pass or an
empty-store check does not establish real authenticated private API access;
that guest verification remains pending.

The repository deploy key remains the Git SSH credential; it is not a `gh`
token. The refresh hook never runs `gh auth setup-git` or changes the clone's
`core.sshCommand`.

The scripts and isolated tests do not establish a live Ubuntu VM, endpoint
reachability, authenticated agent use or a working release signed by the actual
VM key. Those operator-dependent observations remain pending. Production updater
credentials and dashboard execution authority are unchanged.

Sources: [Bombyx tutorial](https://github.com/breki/bombyx/blob/main/docs/tutorial.md),
[Bombyx configuration sample](https://github.com/breki/bombyx/blob/main/config.toml.sample),
[Codex authentication](https://learn.chatgpt.com/docs/auth),
[Codex standalone installation](https://learn.chatgpt.com/docs/cli),
[Claude installation](https://code.claude.com/docs/en/setup),
[Tailscale Ubuntu packages](https://pkgs.tailscale.com/stable/#ubuntu-2404),
[GitHub CLI installation](https://github.com/cli/cli/blob/trunk/docs/install_linux.md).
