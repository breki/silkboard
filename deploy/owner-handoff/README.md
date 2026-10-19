# Private first-owner phone handoff

Keep this bootstrap helper installed for future provisioning; start its
listener only when delivering a first-owner setup link. It refuses an already
registered owner and cannot reset passwords or reopen registration. Other
fresh servers use the same files with different configuration.

The helper retains the Node transport verified during the original phone
rollout. It needs no Cargo or npm packages on the target. Node 24.14.1,
systemd, Tailscale, util-linux `runuser`, and the installed `silkboard-web`
binary must already exist. Node is retained for this small bootstrap tool,
which can run before a Rust toolchain is available. Installation does not
download software, configure application storage, create an owner, issue
publisher credentials, or start/enable the listener.

## Configure and install

Use a trusted checkout on the target Linux guest. Configure persistent private
storage and the owner HTTPS origin through [the rollout](../../docs/dogfood-rollout.md)
first. The application must be healthy and `/api/auth/setup` must report
`waiting` or `available`.

Copy [example.json](example.json) to a private operator file outside the
checkout and edit it for the guest. `origin` and `ownerLogin` are required;
the remaining example fields show defaults. The login is the exact Tailscale
user login authorized to receive the link, independently of the guest's account.
Use an ASCII login; encoded non-ASCII identities are not supported. Paths must be absolute, with no spaces,
`%`, or parent traversal. `applicationUser` must be an existing non-root account.
The resolved account must own the private database and its directory.
Database and origin must match the application settings. `applicationUrl` is
the loopback HTTP origin matching its existing Serve proxy. Ports must differ;
the handoff port must be between 1024 and 65535 and reachable by the owner.

After editing a private `/root/owner-handoff.json`:

```sh
sudo bash deploy/owner-handoff/install.sh \
  /opt/node-v24.14.1/bin/node /root/owner-handoff.json
```

Installation keeps root-owned code under
`/usr/local/libexec/silkboard-owner-handoff`, mode 0700, and mode-0600
configuration at `/etc/silkboard-owner-handoff.json`. The generated service
lives under `/etc/systemd/system` without an enable target. It uses the supplied
application service, state directory and Node path. Reinstallation requires
stopping an active handoff. Treat these inputs as privileged configuration.

## Start, deliver and stop

```sh
sudo systemctl start silkboard-owner-handoff.service
sudo systemctl status silkboard-owner-handoff.service --no-pager
```

Open the configured hostname at the handoff port, for example
`https://dashboard.example.ts.net:8443/`. This ordinary URL contains no secret
and can be shared with the owner. They connect through Tailscale, press
**Continue to setup**, choose a password on the normal application origin,
and sign in. Never request their password or redirected setup link through
chat. Do not press the live button or record a browser trace/redirect for them.

Each start discovers the guest's Tailscale addresses and checks its DNS name,
main HTTPS proxy, free handoff port, absence of Funnel exposure, private database
and pending registration. Requests from the guest itself, loopback, other
accounts, missing identities and forwarding chains are rejected. The root-private
socket prevents local users from forging Serve headers. Address drift before
preparation also causes refusal. GET never prepares a link; an exact-origin
nonce POST does.

The root helper invokes the installed binary as the application user. Its
bounded private stdout pipe carries the raw link into memory, then directly
into the owner's HTTPS redirect. The unit retains `CAP_SETUID` across child
execution under `NoNewPrivileges`; `runuser` drops effective, permitted and
ambient capabilities when switching to the application user. This fixes the
child-switch failure observed during the original rollout. Network restrictions
and application permissions remain in place.

Backend and foreground Serve stop after delivery, twenty minutes or service
stop, removing only their temporary listener. The main Serve configuration and
installed code/configuration remain. Restart an interrupted handoff while
setup is pending; the next button press replaces the pending link. A delivered
link lasts up to one hour, independently of the temporary listener. If
registration may have completed, try normal login first; never erase owner
state to retry setup.

```sh
sudo systemctl stop silkboard-owner-handoff.service
sudo tailscale serve status
```

Confirm the extra listener disappeared and the normal application is healthy.
Do not use `tailscale serve reset` or disable another listener for cleanup.
Once `/api/auth/setup` reports `closed`, future starts safely refuse.

## Remove the installed helper

Stopping is normally sufficient. To remove the tool entirely:

```sh
sudo systemctl stop silkboard-owner-handoff.service
sudo rm -- /etc/systemd/system/silkboard-owner-handoff.service \
  /etc/silkboard-owner-handoff.json
sudo rm -- /usr/local/libexec/silkboard-owner-handoff/{config,server,runtime,main}.mjs
sudo rmdir /usr/local/libexec/silkboard-owner-handoff
sudo systemctl daemon-reload
```

Preserve application storage and owner records. Removal must not delete the
database, reset registration or change the updater.

## Verification boundary

`cargo xtask handoff-test` checks formatting, configuration, transport,
subprocesses, lifecycle and isolated HTTPS mobile-browser behavior. Full
validation includes it, with 90% line/function and 85% branch coverage minimums
for the tested bootstrap modules. The small privileged entry point requires verification
on each target. Browser tests simulate Serve identity and the setup
landing page; they prove the two-origin redirect, not another server's real
identity, sandbox or phone acceptance. Verify the privileged entry point and
installer on each newly provisioned guest.

The initial one-off implementation was verified on production on 2026-10-19:
owner GET succeeded, guest-local/spoofed requests failed, stopping preserved
port 443, and the owner confirmed registration and phone login. A child UID/
capability probe passed with the corrected sandbox. Its temporary deployment
was removed. The configurable repository version is locally verified and
has not been installed on that registered production instance.

See [the setup contract](../../docs/owner-setup.md), Tailscale's
[identity documentation](https://tailscale.com/docs/features/tailscale-serve)
and [foreground lifecycle](https://tailscale.com/docs/reference/examples/serve).
