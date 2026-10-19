# First-owner setup from a phone

Issue [#45](https://github.com/breki/silkboard/issues/45) adds a one-time private
HTTPS registration page. The provisioner prepares a link; the owner opens it
on their phone and sets a password without SSH access. The dashboard accepts
publisher reports only after an owner record exists. Merely visiting the site,
preparing a link, abandoning setup or issuing a publisher token cannot unlock
ingestion. Health/release status remains accessible to the updater.

## Provisioning and private delivery

Install a reviewed release that includes this implementation, then configure
the persistent private `SILKBOARD_DATABASE` and exact HTTPS
`SILKBOARD_OWNER_ORIGIN` as described in [owner login](owner-login.md#server-configuration).
Production storage/service configuration, backups and deployment remain
[#39](https://github.com/breki/silkboard/issues/39). The currently installed
production release does not acquire this feature merely by merging the PR.

The provisioner runs `silkboard-web owner prepare-setup` with those same
database/origin settings. It generates a new random 256-bit credential, stores
only its SHA-256 verification hash and an expiry one hour in the future, and
writes the setup link once to stdout. Stdout must be a private pipe rather
than a terminal. Stderr never contains the link. Do not run this command through
an agent tool, recorder, CI log or analytics system that captures stdout.
Never put the link in the repository, chat, shell arguments or a server file.

For a provisioner using an already approved SSH route, capture that stream in
a new private file on the **provisioner's machine**, outside the checkout.
Use the installed database/origin values; this example assumes the #39 storage
layout has been installed and does not install it:

```sh
umask 077
set -o noclobber
ssh -T APPROVED_SSH_TARGET \
  'sudo -n -u silkboard env SILKBOARD_DATABASE=/var/lib/silkboard/progress/progress.sqlite SILKBOARD_OWNER_ORIGIN=https://silkboard.tailf38f2f.ts.net /opt/silkboard/current/silkboard-web owner prepare-setup' \
  > "$HOME/.config/silkboard-owner-setup.link"
```

Create/control the local configuration directory beforehand. Preserve normal
SSH host-key checking, keep tracing/session recording disabled, and treat the
file as a password. Transfer it directly to the owner's private password vault
or another approved encrypted channel accessible on the phone, without copying
it through agent chat or logs. Delete the transfer file and vault entry after
registration. The owner needs the private HTTPS site reachable on their phone,
not shell access. Actual production delivery is a rollout step, not evidence
provided by these local instructions.

Repeating preparation replaces the pending hash and immediately invalidates
the old link. If delivery fails or the hour expires, prepare a replacement and
deliver it privately again. No server copy of the raw credential is retained.
After registration or an operator password setup/reset, preparation refuses
to reopen first-owner setup. There is no public signup or web password reset.

### Temporary phone handoff

For the initial rollout, the owner selected a temporary private HTTPS handoff
restricted to their Tailscale account. This delivers the existing setup
credential; it does not replace registration. Preparing the helper does not
authorize enabling production storage or starting it.

The [reusable provisioning helper](../deploy/owner-handoff/README.md) uses a
root-private Unix socket behind a separate Tailscale Serve HTTPS listener,
defaulting to port 8443. It accepts only the exact
owner login from Serve's identity header and refuses missing identities, other
accounts and Funnel requests. It also requires Serve's authoritative source IP
and denies the guest's own addresses, loopback and forwarding chains. The
initial production guest's Tailscale account matched the owner, so the login
check alone would incorrectly admit local application processes through Serve. Each start
discovers the current guest addresses; drift before preparation causes refusal.
Other local users must not reach the backend socket, since direct callers
could forge those headers. Serve strips incoming identity headers and supplies its authenticated identity; tagged devices do
not receive a user identity. See the
[installed-version proxy implementation](https://github.com/tailscale/tailscale/blob/v1.104.1/ipn/ipnlocal/serve.go).

GET displays a button without preparing a link. An exact-origin POST with a
fresh in-memory form nonce runs the installed setup command as the application
user. Its bounded stdout travels through a private pipe into helper memory;
only an exact application-origin setup link is accepted. The authenticated
browser receives an HTTPS redirect to that link. Nothing prints or writes the
raw credential. Responses prohibit caching and framing; no request or response
logging is enabled. The secret-free form uses a same-origin referrer policy so
browser navigation preserves its Origin check; the redirect uses no-referrer.
The form policy permits only the handoff and exact application origin, since
the redirect crosses from port 8443 to 443.
The existing setup page removes the fragment and requires an owner password.

The helper closes after one delivery or twenty minutes. Foreground Serve ties
the extra listener to the helper service's lifetime, including interruption,
without resetting or replacing the application's port 443 configuration. See
[Serve lifecycle documentation](https://tailscale.com/docs/reference/examples/serve).
If delivery or registration is interrupted, the operator prepares a replacement
handoff, invalidating any previous pending setup link when the button is pressed.
A delivered link is not evidence of completed registration. Installation,
real identity checks, phone registration and temporary listener removal still
require live verification under the rollout authorization. The initial one-off
production handoff and phone registration/login were verified on 2026-10-19;
the configurable helper is retained for future provisioning and remains
uninstalled on that registered instance.

## Phone flow and secret handling

Open the private link in an HTTPS browser. Its secret is in the `#setup=...`
fragment, which is excluded from HTTP request URLs. The client replaces the
current history entry with `/setup` before rendering the form and keeps the
credential only in memory. It never appears in form fields, cookies, local or
session storage, or `history.state`. Reloading/navigation discards it; reopen
the original private link if setup has not been completed. The page uses
`Cache-Control: no-store`, `Referrer-Policy: no-referrer`, denied framing and
a same-origin content policy, with no third-party scripts or analytics.

A setup link is a secret even with these controls: browser/OS link handling,
extensions, screenshots and the chosen delivery channel are outside the app's
control. Use a private trusted browser/device, avoid bookmarking or sharing the
link, and clear any delivery history after use. Do not enable proxy body logging,
browser traces or HTTP capture that records setup POST bodies or passwords.
The application has no request/access/body logger. The fragment avoids normal
URL access logs; it is sent only in the registration JSON body over HTTPS.

Enter and confirm a long unique owner password, with the existing 12–1,024-byte
limit and no control characters. Submission clears both inputs. A missing,
expired, replaced or invalid link gives a safe explanation; exhausted attempt
budgets ask you to wait a minute. An interrupted response may have registered
the owner: follow **Already registered? Sign in** or the normal site and try
the chosen password. After success, the link is consumed and **Sign in to
Silkboard** uses normal [owner login](owner-login.md). The separate
[publisher credential manager](publisher-tokens.md) then handles agent tokens.

## Security and durable state

- `GET /api/auth/setup` returns only `waiting`, `available` or `closed`.
  It does not validate, return or consume a credential. GETs/link scanners
  cannot register an owner.
- `POST /api/auth/setup` requires JSON `credential` and `password`, one exact
  configured `Origin`, `X-Silkboard-Setup: 1`, and same-origin/none fetch
  metadata when supplied. JSON is limited to 4,096 bytes; unknown fields fail.
  The non-simple header, non-ambient credential and strict origin protect the
  unauthenticated registration mutation against CSRF. CORS is not enabled.
- Setup and login share five attempts per process per 60-second window and
  two hashing slots. Invalid setup credentials are checked before expensive
  Argon2 work. Hashing uses the existing salted Argon2id policy; blocking
  workers keep it off the asynchronous request loop. No password is logged.
- An immediate SQLite transaction rechecks the credential, expiry and closed
  state after hashing, then inserts the sole owner and consumes/closes setup
  together. Concurrent requests or replacement/reset during hashing cannot
  overwrite a winner or revive an older credential. Success is 204, with no
  automatic browser session; normal login follows. Invalid links return 401,
  closed setup 410, invalid input 400, forbidden origin/header 403, exhausted
  budgets 429, and storage failures a generic 503.
- Schema 3 adds a singleton setup row and permanent closure flag. Schema 1/2
  migrations retain progress, publishers and browser sessions. An existing
  owner starts closed. Reopening the database, replacing an expired pending
  link, resetting a password or restarting the service cannot reopen closed
  registration. Binaries supporting only older schemas refuse schema 3;
  backup/rollback needs a compatible database or reviewed migration plan.
- Every ingestion path checks owner existence in its write transaction before
  authenticating/storing an event. An uninitialized database returns 503 even
  with a valid publisher token. Owner cookies alone still cannot publish;
  publisher tokens cannot register owners, read feeds or manage the updater.

## Verification boundary

`cargo test -p silkboard-web` checks unauthorized claims, hash-only state,
expiry/replacement/replay, concurrent registration over separate SQLite
connections, publisher gating, migration, reopening/reset and failure handling.
Frontend tests cover missing links, errors, password clearing and navigation.
The isolated Chromium suite uses a second disposable backend and HTTPS proxy
to exercise phone-sized registration, fragment/history removal, reload, wrong
credentials, transition from blocked to accepted publishing and normal secure
cookie login. These are local fixtures, not production owner setup or phone
acceptance. Production registration, release/backup behavior and owner phone
verification remain #39 and require the normal deployment authorization.

References: [MDN URI fragments](https://developer.mozilla.org/en-US/docs/Web/URI/Reference/Fragment),
[MDN replaceState](https://developer.mozilla.org/en-US/docs/Web/API/History/replaceState),
and [OWASP reset-token guidance](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html).
