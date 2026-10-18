# Manage publisher credentials from a phone

Issue [#44](https://github.com/breki/silkboard/issues/44) adds credential
management to the signed-in owner page. After [first-owner setup](owner-setup.md),
sign in and choose **Manage publisher credentials**. This is a local/reviewed
implementation; production installation and credential delivery remain
[#39](https://github.com/breki/silkboard/issues/39), through the usual release
and deployment procedure.

## Issue and save

Enter the project's lowercase slug (1–64 letters, digits or hyphens) and a
unique, descriptive installation label. Labels contain 1–256 ASCII graphic
characters without spaces, and remain reserved after revocation. For example,
use project `silkboard` and label `development-vm-codex-1`. The project is
created if it does not already exist. Use **Load credentials** to inspect a
project without issuing anything; listings contain labels and Active/Revoked
state only, twenty entries at a time.

Choose **Issue token** once. A new 256-bit random project-scoped publisher
credential appears in **New publisher token**. This is the only issuance
response that reveals it: the server stores only its SHA-256 hash. Neither the
list nor another API can retrieve it later. The form is disabled while the
token is shown so another issuance cannot accidentally replace this display.

Deliberately choose **Copy token** or **Download token file**, then save it
privately. Clipboard access needs HTTPS/browser permission; if refused, select
the field and copy it manually or use the download. The downloaded
`silkboard-publisher.token` contains the 64 hexadecimal characters and one final
newline. The fixed filename contains no project/label or path supplied by a
caller. The browser creates a temporary local Blob URL and releases it after
requesting the download; the secret is never put into a navigation URL.

Choose **I’ve saved it** to clear the display. Closing the manager, navigation,
reload, logout or detected session expiry also discards it. Nothing saves it
in local/session storage, cookies, history, an outgoing queue or server logs.
The page clears the credential on `pagehide`, including back/forward-cache
navigation; it does not promise to erase browser/OS memory or extensions.
Owner pages and credential API responses are `no-store`, deny framing and
use the existing same-origin content/referrer protections. Do not enable body
logging, analytics or browser traces that capture secret responses/fields.

On the publishing machine, install the saved value using the
[private configuration procedure](publishing-cli.md#private-configuration):
the token file must be mode 0600 in an owner-controlled mode-0700 directory.
Browser downloads do not establish those target permissions. Transfer it using
an approved private channel or password vault, without pasting it into chat,
URLs, command arguments or recorded terminals. Issuing a credential does not
install it on a machine, enable production storage or authorize execution.
It cannot read feeds, manage other credentials, access repositories/VMs or
use the separate updater's release credentials.

## Recovery and revocation

If issuance times out or its response is lost, do not assume nothing happened.
**Load credentials** for that project. If the installation label exists and
you did not save its token, revoke it and issue a replacement under a new
label. There is no reveal/recover action and no automatic issuance retry.
Duplicate labels, including revoked ones, report a conflict without replacing
the earlier credential.

If issuance succeeds but refreshing the list fails, save the displayed token;
the credential was issued successfully. A confirmed revocation likewise stays
revoked in the displayed metadata if its refresh fails. Reload the list later
to obtain current metadata.

Choose **Revoke** beside the correct project/installation, then **Confirm
revocation**. Revocation is immediate and idempotent; already accepted progress
and session history are retained. The installation's next publish is rejected
with 401. If the response is lost, load the metadata again and check the state;
retrying revocation is safe. Cancelling the confirmation changes nothing.
Install a newly issued credential privately if that machine should resume
publishing. Reusing or deleting an old label is not rotation.

An expired owner session removes private controls and returns to sign-in.
Owner password reset/logout invalidates its authority; a publisher credential
never substitutes for the owner's browser login. Failed requests use fixed
messages rather than echoing credentials, SQL or raw server errors.

## HTTP and storage contract

All routes live inside the existing protected owner router:

| Route | Result |
| --- | --- |
| `GET /api/owner/projects/PROJECT/publishers?after=LABEL` | `{items:[{label,revoked}],next_cursor}`; 20-item keyset pages ordered by label |
| `POST /api/owner/projects/PROJECT/publishers` | JSON `{label}`; 201 with `{token}` only |
| `POST /api/owner/projects/PROJECT/publishers/revoke` | JSON `{label}`; idempotent 204 |

The optional `after` is an installation-label cursor, never a token. New or
empty projects return an empty list. Invalid project/label inputs return 400;
duplicate issuance returns 409; missing/invalid/expired owner sessions return
401. Origin/CSRF or fetch-metadata rejection returns 403, and storage/admission
failures return a fixed 503. JSON mutations are bounded to 4,096 encoded bytes
and reject unknown fields. These responses never include token hashes.

Unsafe requests require the exact configured HTTPS `Origin` and the owner's
`X-Silkboard-CSRF` value. Publishers and unauthenticated visitors cannot list,
issue or revoke. Mutations reauthenticate the owner session inside the same
immediate SQLite transaction used to issue/revoke; a completed logout/reset
cannot be bypassed by an earlier successful HTTP guard. At most eight owner
database operations are admitted to blocking workers. Existing publisher
generation, label uniqueness and revocation semantics are shared with the
operator CLI. No schema migration, new dependency or token self-updater is
introduced; schema 3 and normal backup/restore requirements remain unchanged.

## Verification boundary

Backend tests exercise the full issue/list/revoke lifecycle, protected routes,
forged publisher-as-owner sessions, Origin/CSRF, pagination, malformed requests,
concurrent duplicate labels, transaction-time logout and retained progress.
Frontend tests cover deliberate saving, cancellation, unavailable clipboard or
download, uncertain outcomes, safe text, pagination and late results after
navigation. The isolated HTTPS browser suite exercises phone-sized controls,
actual clipboard/download bytes, one-time display and publishing/revocation.
Local checks do not establish production phone access or credential installation;
the initial live dogfood rollout remains #39.

Browser reference APIs: [MDN Clipboard.writeText](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard/writeText),
[Blob URLs](https://developer.mozilla.org/en-US/docs/Web/API/URL/createObjectURL_static),
and [pagehide](https://developer.mozilla.org/en-US/docs/Web/API/Window/pagehide_event).
