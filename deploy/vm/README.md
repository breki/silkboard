# Production VM bootstrap

For milestone 2 owner registration, the provisioner must prepare/deliver the
[one-time private setup link](../../docs/owner-setup.md#provisioning-and-private-delivery)
after installing a reviewed compatible release and persistent storage. The
owner can then register from a phone without SSH. Publishing remains locked
until initialization succeeds. Production service/storage installation and
real owner acceptance remain #39; these bootstrap instructions do not enable
them implicitly or grant a new deployment authorization.

Operator bootstrap procedure for issue #5. Automatic polling, eligibility,
recovery, and retries have a separate [updater procedure](../../docs/automatic-upgrades.md)
for issue #6. The [in-repository toolkit](../../docs/deployment-toolkit.md)
adds operator policy, structured reports and state-version compatibility;
read its upgrade notes before installing new updater code. Admission and polling are verified; live upgrade and recovery
demonstrations remain pending. Do not install
the host/guest-agent scripts in the dashboard or build account.

## Boundaries and prerequisites

The target is malina, Debian 13 ARM64, with working QEMU/KVM, libvirt,
virt-install, cloud-localds, and passt. The operator needs existing SSH/sudo
access to malina; the local transport additionally needs Bash, jq, base64,
and sha256sum. No host bridge, port forwarding, DNS change, or host reboot is
required. The operator-only QEMU guest agent has guest-root authority.

Create a separate `silkboard` domain with two shared vCPUs, fixed 2 GiB RAM
(no balloon device), and an independent 40 GiB qcow2 disk including 4 GiB
swap. The trial remains untouched. Creation refuses an existing domain or
image directory, invalid public key, wrong image checksum, or insufficient
host resources. Re-running a completed creation fails safely instead of
replacing a guest. Inspect partial failures; there is no automatic deletion.

The `silkadmin` account is the operator, with SSH key access and guest sudo.
`silkbuild` has no SSH key, sudo rule, runtime write permission, or VM-control
socket; it builds sequentially with one Cargo job. The `silkboard` service
account has no login shell, source checkout, credentials, or VM authority.
Root installs runtime artifacts read-only and systemd hides build/trust paths.
This is guest/application separation, not a claim of verified host confinement:
the existing host reported no libvirt security driver during the trial.

## 1. Create the guest

Use Debian image build `20261015-2632` from the official
[Debian cloud directory](https://cloud.debian.org/images/cloud/trixie/20261015-2632/).
Download `debian-13-generic-arm64-20261015-2632.qcow2`, or reuse the unchanged
trial base after the bootstrap verifies its pinned SHA-512. Production gets
its own copy, so later trial cleanup cannot break its backing chain.

Copy `host-create.sh`, `cloud-init.yaml.in`, and the operator's ED25519 SSH
**public** key into a dedicated operator directory on malina. Never copy a
private key. Run on malina with the actual paths:

```sh
sudo ./host-create.sh --check --image /path/to/debian-image.qcow2 \
  --ssh-public-key ./operator.pub
sudo ./host-create.sh --apply --image /path/to/debian-image.qcow2 \
  --ssh-public-key ./operator.pub
```

Libvirt autostart is enabled for this guest. Cloud-init installs build
prerequisites, the QEMU guest agent, and guest swap. Allow first-boot package
installation to finish. A guest-agent-not-ready error is not completion.

## 2. Bootstrap the guest

Run remaining commands from the Silkboard checkout on the operator workstation.
The transport uses only the fixed `malina` SSH target and `silkboard` domain;
JSON is sent on stdin. File uploads are limited to a root-only bootstrap
folder and checked against a guest SHA-256. Commands time out after 30 minutes;
a timed-out guest process may still run, so inspect before retrying.

```sh
deploy/vm/guest-agent.sh exec /usr/bin/cloud-init status --wait
for file in verify-release-signers.sh guest-bootstrap.sh guest-build.sh guest-stage.sh guest-activate-initial.sh silkboard.service; do
  deploy/vm/guest-agent.sh put "deploy/vm/$file" "$file"
done
deploy/vm/guest-agent.sh put .github/release-signers release-signers
deploy/vm/guest-agent.sh exec /bin/bash /tmp/silkboard-bootstrap/guest-bootstrap.sh \
  /tmp/silkboard-bootstrap/release-signers /tmp/silkboard-bootstrap/silkboard.service
deploy/vm/guest-agent.sh exec /usr/bin/install -d -m 0755 /usr/local/libexec
deploy/vm/guest-agent.sh exec /usr/bin/install -m 0755 \
  /tmp/silkboard-bootstrap/guest-build.sh /usr/local/libexec/silkboard-build
```

Bootstrap checks the exact approved owner/VM release signer fingerprints, installs a separate
root-owned trust file, Node 24.14.1 and Rust 1.93.1, and Tailscale from its
signed Debian repository. Node and rustup downloads have pinned SHA-256 values;
Rust is installed under the build account, not root. See official
[Node downloads](https://nodejs.org/dist/v24.14.1/SHASUMS256.txt) and
[rustup archive](https://static.rust-lang.org/rustup/archive/1.29.1/aarch64-unknown-linux-gnu/rustup-init.sha256).
For long commands, redirect guest output into a durable `/var/log/` file and
inspect its tail; QEMU may truncate captured output.

## 3. Install an eligible initial release

Before transferring source, verify its signed tag using the independently
approved signer and read back successful full release-suite evidence from
GitHub. Bind repository `breki/silkboard`, workflow ID `387356305`, path
`.github/workflows/release.yml`, push event, tag, and exact peeled commit.
Follow [release evidence](../../docs/releases.md). Unavailable or incomplete
checks block installation. The initial operator verifies these gates; the
build script does not query GitHub and must not be mistaken for an updater.

For `v0.1.1`, commit `c68a53902d8e9552abee2d3488645800f856986e`:

```sh
mkdir -p .local
git bundle create .local/silkboard-v0.1.1.bundle refs/tags/v0.1.1
deploy/vm/guest-agent.sh put .local/silkboard-v0.1.1.bundle source.bundle
deploy/vm/guest-agent.sh exec /usr/bin/install -o silkbuild -g silkbuild -m 0644 \
  /tmp/silkboard-bootstrap/source.bundle /var/lib/silkboard-build/source.bundle
deploy/vm/guest-agent.sh exec /usr/sbin/runuser -u silkbuild -- \
  /bin/bash /usr/local/libexec/silkboard-build /var/lib/silkboard-build/source.bundle \
  v0.1.1 c68a53902d8e9552abee2d3488645800f856986e
deploy/vm/guest-agent.sh exec /bin/bash \
  /tmp/silkboard-bootstrap/guest-activate-initial.sh \
  v0.1.1 c68a53902d8e9552abee2d3488645800f856986e
```

The Git bundle carries source/history and the signed tag, with no GitHub
credential. The builder checks the trusted signature, signed tag name/direct
target and exact commit before compiling; release-check also checks the
application version. Source and output remain in the private build directory.
Activation is operator-only and refuses to replace any existing release.
Artifact data is read as the build user, snapshotted, restricted to regular
files, and normalized to root-owned ordinary modes. A failed health check
removes the initial current symlink and disables the service, retaining logs
and staging for inspection. This is not upgrade rollback verification.

## 4. Private access and reboot verification

Authenticate the new guest interactively with Tailscale; do not put an auth
key in cloud-init, Git, or logs. Use `tailscale up --hostname=silkboard` in the
guest, complete the displayed login, and verify the actual node name. If the
requested name is unavailable, ask the owner before choosing another.
Confirm `tailscale status --json` reports `BackendState: Running` before
rebooting or restarting the guest. A reboot during pending authentication can
invalidate the interactive login attempt.

Obtain `/etc/ssh/ssh_host_ed25519_key.pub` through the trusted guest-agent
channel. Use that key in a dedicated operator known-hosts file before the first
SSH connection as `silkadmin`; do not disable host-key checking. Tailscale SSH
is not enabled: normal OpenSSH uses the operator public key.

In the guest, run `tailscale serve --bg http://127.0.0.1:3000`, then verify
certificate-validated HTTPS at the guest's actual Tailscale DNS name. The
[Serve documentation](https://tailscale.com/docs/reference/tailscale-cli/serve)
explains its persistent background configuration. Do not enable Funnel.

Record `/health`, `/api/status`, the rendered page, service security settings,
exact release identity, guest/host memory and disk, swap and OOM counters.
Reboot only the guest, verify its boot ID changes, and repeat strict SSH,
HTTPS, identity and health checks. Verify libvirt autostart, guest application
and tailscaled enabled state. A host reboot is outside this test.

Keep the initial release and logs while upgrade/recovery verification is pending.
Do not delete the trial as an implied provisioning step; cleanup is issue #2.

## Production build-cache helper

The [dependency-reuse guide](../../docs/production-build-cache.md) describes
the updated helper source, private cache and operator trial/rollout. A normal
application release does not install that helper; measure ARM64 behavior and
review the narrow operator update before changing an existing deployment.
