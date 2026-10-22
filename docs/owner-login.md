# Owner authentication

The backend for [#35](https://github.com/breki/silkboard/issues/35) provides
single-owner password login, durable browser sessions and a protected owner
API namespace. The [owner feed](owner-feed.md) provides the login form and
protected project/session/history reads for #38.
The [publisher credential manager](publisher-tokens.md) adds owner-only
issuance, metadata and revocation using the same login and CSRF boundary.
Agents retain separate publish-only credentials. This does not
install production credentials or enable a production service.

## Operator setup and reset

First-owner registration can use the [one-time setup page](owner-setup.md)
from a phone. Its provisioning-prepared link is separate from login, publisher
tokens and updater credentials. The private-pipe command below remains an
operator setup/reset path and permanently closes first-owner registration.

Use the [private database setup](progress-ingestion.md#local-setup-and-credentials).
The same `SILKBOARD_DATABASE` stores progress and owner authentication. Build
the binary, then set the sole owner's password from a private pipe in an
owner-controlled terminal:

```sh
cargo build --locked -p silkboard-web
export SILKBOARD_DATABASE="$PWD/.local/progress/progress.sqlite"
set -o pipefail
systemd-ask-password --echo=no 'Silkboard owner password' |
  ./target/debug/silkboard-web owner set-password
```

`systemd-ask-password` hides input and sends its result directly through the
pipe. The password never belongs in a command argument, environment variable,
URL, shell history, chat or agent tool input/output. Do not run this command
through a session recorder that captures private input. A password manager
may supply the private pipe instead. Direct terminal input is refused by the
binary to avoid echoing the password. There is no public signup or web
password-reset endpoint.

Choose a long unique passphrase; input must contain 12–1,024 UTF-8 bytes with
no control characters. The pipe may end with one LF or CRLF. Input reading is
bounded and excess data is rejected. The command prints only a completion
message, never the password. Repeating it resets the password and atomically
revokes every browser session, while retaining all progress and publisher
credentials. Invalid input leaves existing credentials and sessions intact.
Losing the password requires operator access to this reset command.

Passwords are salted with 16 random bytes and hashed using Argon2id version
19, 64 MiB memory, three iterations, one lane and a 32-byte output. Only the
PHC password hash is stored. Browser session tokens contain 32 random bytes;
only their SHA-256 hashes are stored, separately from publisher hashes.

## Server configuration

Set `SILKBOARD_OWNER_ORIGIN` to the exact serialized HTTPS origin used by the
browser, for example `https://silkboard.example.test`. Use a lowercase
host, no trailing slash/path/query, no credentials and no default `:443` port.
A nondefault numeric HTTPS port is supported. Both database and origin are
required to enable owner HTTP routes. Invalid configuration fails startup;
an owner without an initialized password cannot log in.

The Axum listener remains loopback HTTP behind the trusted HTTPS proxy;
neither forwarded host headers nor request input select the trusted origin.
Expose authentication only over HTTPS. For local browser development use an
HTTPS origin/proxy rather than weakening cookies for HTTP. The existing
production service configuration is unchanged; persistent storage, secure
credential entry from a phone and rollout verification belong to
[#39](https://github.com/breki/silkboard/issues/39).

## Browser API

- `POST /api/auth/login`: JSON `{"password":"..."}` and an exact matching
  `Origin` header. Success is 204 and sets a new browser cookie. Unknown
  fields, malformed JSON, missing JSON content type or oversized bodies
  return a generic 400; wrong/unset passwords return 401. JSON requests are
  bounded to 4,096 encoded bytes, and passwords have the limits above.
- `GET /api/owner/session`: requires the owner cookie. Returns only
  `{"authenticated":true,"csrf_token":"..."}`. The CSRF token is bound to
  that session; it is not a publisher token or an execution capability.
- `POST /api/owner/logout`: requires the owner cookie, exact matching
  `Origin` and the session's `X-Silkboard-CSRF` header. Success is 204,
  deletes that server session and expires the cookie.

The cookie is `__Host-silkboard-owner`, with `Secure`, `HttpOnly`, `Path=/`,
`SameSite=Strict`, no `Domain`, and a 12-hour maximum age. Sessions also
expire absolutely after 12 hours in SQLite, without sliding renewal. Login
always generates a fresh token. Logout revokes that session; a password
reset revokes all sessions, including a login still verifying the old
password when the reset committed. Valid sessions survive server restart.
Expired records are removed during a later successful login.

Login permits five attempts per fixed 60-second window starting at
the first attempt after a window reset; the initial window starts at process
configuration. The counter is shared across clients in this process and is
not persisted. At most two password verifications run at once (128 MiB
combined hashing memory). Exhausted budget or hashing slots return 429 with
`Retry-After: 60`; successful attempts also consume budget. HTTP handlers use
blocking workers for hashing and SQLite, with eight admitted owner database
operations. Saturation/storage/hash failures return generic 503 responses;
no secrets or SQL are included. Restarting resets only the in-memory budget,
not session expiry or revocation.

Every request under `/api/owner` passes the authentication guard, including
unknown routes. Feed/session reads are registered inside that
protected router. Publisher bearer tokens are ignored there; supplying one
as an owner cookie also fails. Conversely, an owner cookie cannot publish
progress without a separate publisher bearer token. Health/status routes
remain public for the updater. Ingestion now also requires an initialized
owner, including when configured without owner browser routes.

Cross-origin/opaque-origin logins fail. Authenticated reads reject mismatched
Origin headers and cross-site fetch metadata; unsafe requests additionally
require an exact origin and a constant-time CSRF-token comparison. Duplicate
owner cookies and duplicate mutation Origin headers are rejected. CORS is
not enabled. Authentication responses use `Cache-Control: no-store` and
`X-Content-Type-Options: nosniff`. Browser login grants no repository, VM,
updater or agent execution permissions.

## Data compatibility and verification

Schema 2 added the singleton owner record and browser sessions in the same
transaction used to initialize/migrate the database. Existing schema-1
progress and publishers are retained. The current schema 3 additionally stores
the one-time setup hash/expiry and permanent closure state. Existing owners
migrate closed; newer schemas are refused. Binaries supporting only older
schemas cannot open schema 3; restoring an older application
that uses this database needs a compatible backup or a reviewed migration
procedure. Do not automatically downgrade or delete authentication data.
The [data guide](application-data.md) provides private backup/new-path restore;
restores retain passwords/history but revoke restored publisher credentials,
browser sessions and pending setup links. Production upgrade/restore evidence
remains #39 work.

Run `cargo test -p silkboard-web` for authentication, origin/CSRF rejection,
publisher isolation, reset/logout/expiry, restarts, malformed hash handling,
operator piping, migration and existing ingestion/status regression tests.
These tests do not prove production login, real-browser cookie handling or
phone usability; those checks accompany the frontend and production rollout.

Reference interfaces: [RustCrypto Argon2](https://docs.rs/argon2/0.5.3/argon2/),
[MDN cookie guidance](https://developer.mozilla.org/en-US/docs/Web/Security/Practical_implementation_guides/Cookies),
and [systemd-ask-password](https://www.freedesktop.org/software/systemd/man/257/systemd-ask-password.html).
