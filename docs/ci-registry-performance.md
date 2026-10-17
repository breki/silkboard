# Live dependency-registry check performance

Issue [#23](https://github.com/breki/silkboard/issues/23) authorizes bounded
concurrent requests while retaining live dependency evidence and policy.

## Behavior and limits

The gate uses at most four workers per batch and deduplicates npm metadata
requests across locked versions of each package. Workers verify their own
responses before discarding them. Any failed request or verification stops
subsequent batches, after all workers in the current batch finish. HTTP 429
is fatal and is never retried automatically. Connection/request timeouts
remain 10/30 seconds; no passing verdict or publication-date cache is added.
Curl requests compressed responses. When both registries have work, each
batch interleaves one crates.io request with up to three npm requests so
npm verification can progress during crates.io pacing.

The [crates.io API policy](https://crates.io/data-access#api) requires at most
one request per second. This is enforced within each gate invocation; separate
CI runs do not coordinate their shared egress rate. Npm requests use the
four-worker bound. Crates.io pacing can offset concurrency savings on fast
networks. Batch barriers also wait for the slowest request; four concurrent
large npm responses can use more memory than sequential verification.

## Measurements

The sequential baseline and replacement use unchanged Cargo/npm lockfiles
and live registries on the same Linux development machine, with already-built
debug xtask binaries. GNU `time -v` wraps the entire `dep-age-check`, including
Cargo metadata, curl subprocesses, JSON parsing and verification. This is a
single before/after observation, not a controlled CI benchmark or a promised
speedup. Network conditions and available CPU affect the result.

The old sequential implementation made 340 requests: 71 crates.io versions
and 269 npm packages covering 277 npm versions, for 348 versions in total.
That count follows its request loop and the unchanged lockfiles; the baseline
was not instrumented. The replacement's curl wrapper records actual requests.
GNU time's maximum RSS is a process high-water mark, not the sum of every
concurrent process or a guaranteed memory ceiling.

Measurements on 2026-10-17:

| Implementation | Elapsed | User + system CPU | Maximum RSS (KiB) | Requests |
| --- | ---: | ---: | ---: | ---: |
| Original sequential, unpaced | 131.55 s | 35.88 s | 275,388 | 340 inferred |
| Four-worker batches, paced, registries separate | 135.39 s | 41.78 s | 295,716 | 340 observed |
| Final: paced, interleaved, compressed | 82.71 s | 36.34 s | 206,356 | 340 observed |

The final run verified all 348 versions with 340 distinct requests: 71
crates.io and 269 npm. Against the original local observation, elapsed time
fell by 48.84 seconds (37.1%) and maximum RSS by 25.1%; total CPU time rose
by 0.46 seconds (1.3%). These results combine interleaving and compression
with bounded concurrency; they do not isolate each change's contribution or
establish a general memory reduction. An intermediate compressed run failed
on a crates.io DNS error and returned failure; it is excluded from timings.

The earlier warm GitHub CI gate took about 58 seconds in
[run 39057089759, attempt 2](https://github.com/breki/silkboard/actions/runs/39057089759/attempts/2).
That implementation had no crates.io pacing. With 71 crates.io requests,
the new gate needs at least 70 seconds between its first and last request
starts, plus completion of outstanding work. It cannot beat that particular
CI sample while honoring the API limit. No reduction in total GitHub runner
time is claimed from these local measurements. A database-dump or different
metadata-source design would require separate investigation; the live
per-version evidence policy is retained here.
