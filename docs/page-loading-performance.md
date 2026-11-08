# Page-loading performance

Issue [#153](https://github.com/breki/silkboard/issues/153) follows the owner's
report of slow data loading on Agents, Events and Workflows. The implemented
change reduces frontend transfer work: fingerprinted public JS/CSS assets are
cached, and gzip siblings are generated during the frontend build. HTML and
private API responses retain `Cache-Control: no-store`; authentication and
reporting permissions are unchanged. This is local verification, not a live
owner-device improvement claim.

## Evidence and scope

Read-only production inspection on 2026-11-08 found v0.15.0 at
`f35c8b8443a48088ddcd5ef51498af9ea58791b8`. Guest loopback health, status and
HTML responses completed in approximately 1.2–2.0 ms; fresh private HTTPS
requests from the development machine took 0.22–0.30 s including TLS. The
guest had low load, about 1.7 GiB available memory and no swap use. These are
public endpoint/resource observations, not authenticated data-query or
owner-device measurements. No production configuration, data or service was
changed, and no dashboard content or owner credentials were read.

The baseline app used full document navigation for primary links, and its
owner router applied `no-store` even to fingerprinted frontend assets. A real
Chromium/HTTPS fixture reproduced repeated downloads of about 270 KB per
navigation. Its isolated owner account and reports are self-authored test
data, with a separate server/owner state so other tests cannot exhaust its
login-attempt budget. The controlled network uses 75 ms latency and 125,000
bytes/second download throughput (1 Mbps), not an inferred owner network
configuration.

| Data-ready observation | Before | After |
| --- | ---: | ---: |
| Cold Agents load | 2,762 ms | 1,286 ms |
| Navigate to Events | 3,090 ms | 616 ms |
| Navigate to Workflows | 3,148 ms | 444 ms |
| Cold asset transfer, including Resource Timing overhead | 270,601 bytes | 86,426 bytes |
| Asset transfer on subsequent Events/Workflows navigation | 270,601 bytes each | 0 bytes |

The comparison uses the same disposable fixture (25 event reports and four
workflow snapshots) and waits for loaded Agents, Events and Workflows data.
The baseline uses the original server with the same final frontend bytes;
that server neither negotiates gzip nor permits cached asset reuse. The
final implementation is restored before normal validation. These are diagnostic
samples, not latency budgets or statistical guarantees. The regression gates
observable cache/compression
behavior rather than elapsed-time thresholds. `encodedBodySize` remains
nonzero for cached assets; `transferSize` is zero when no network transfer
occurs. A fake browser clock is intentionally not used for these measurements.

The browser regression also waits for an actual ten-second workflow poll and
checks that private data stays uncacheable while public assets are not fetched
again. Initial list polling starts immediately; ten seconds is its refresh
cadence, not an intentional initial loading delay. Session verification still
precedes mounting the data view, and primary links still use document
navigation. Removing those steps was not needed for this measured transfer fix.

## Synthetic query check

An explicit diagnostic test measured authenticated owner route handling with
20 synthetic sessions and retained event histories. Each mean covers five
requests, including session verification, query execution and response decode,
against an in-memory SQLite store in the normal test profile.

| Events | Agents mean | Events mean | Agents project choices mean |
| ---: | ---: | ---: | ---: |
| 100 | 2.742 ms | 2.022 ms | 1.034 ms |
| 1,000 | 8.882 ms | 1.528 ms | 1.994 ms |
| 10,000 | 91.174 ms | 1.699 ms | 16.699 ms |

The Agents full-history ranking and shared SQLite mutex remain scaling leads.
This check does not simulate a large workflow snapshot history, concurrent
publishing, disk contention or the production ARM64 CPU. It does not prove
the owner's authenticated requests are fast. No query/schema optimization is
claimed: under the measured browser conditions, asset transfers alone already
account for a substantial repeatable navigation delay.

## Cache and build contract

Only successful public `/assets/` JS/CSS responses with Vite's default eight
URL-safe fingerprint characters receive
`Cache-Control: public, max-age=31536000, immutable`. Failed paths,
unversioned files and private responses keep `no-store` in the owner app.
The asset service negotiates gzip, sends `Vary: Accept-Encoding`, and falls
back to the original file when no gzip sibling exists or gzip is unacceptable.
HEAD, ranges and conditional revalidation retain their HTTP semantics.

The Vite build uses Node's built-in zlib after bundle writing to produce
`.js.gz` and `.css.gz` siblings from final disk bytes, including Vite's lazy
import maps and finalized CSS. A regression decompresses every generated
JS/CSS sibling and checks byte equality with the original. Both original and
compressed files are included by the existing
release-directory copy. Compression happens during the build, not on the Pi
per request. There is no new compression dependency or private-response
compression policy. Older frontend directories without gzip siblings remain
servable. Fingerprinted URLs allow a new release or rollback to select its
own assets while HTML is fetched fresh.

## Reproduce and verify

Install locked frontend dependencies and build assets as in the
[development guide](development.md). From `frontend/`, run the focused browser
check with a free isolated port block:

```sh
CI=true SILKBOARD_E2E_PORT_BASE=39040 npm run test:browser -- page-loading.spec.js
```

It starts only disposable local services and prints/attaches timing and byte
counts without recording report content or credentials. The first asset load
must transfer bytes; repeat navigation must transfer none. Built JavaScript
must execute with gzip, and private APIs must still send `no-store`.

From the repository root, run the optional synthetic query diagnostic:

```sh
cargo test --locked -p silkboard-web --test feed \
  measure_owner_queries_with_retained_history -- --ignored --nocapture
```

This diagnostic is deliberately outside normal timing gates; its initial run
is recorded above. Use the physical pinned Node runtime on PATH when commands
enter disposable repositories, and the existing `CI=true` frontend worker
limits for full validation. Existing route, authorization, filter, pagination
and workflow browser suites remain required.

After a separately authorized release/deployment, owner-assisted timing on the
affected device should compare cold load, repeat navigation and live data
refresh without sharing cookies or dashboard content. Local results do not
establish that production rollout or acceptance.
