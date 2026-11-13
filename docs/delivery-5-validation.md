# Delivery 5 validation record

Recorded on November 13, 2026 for **v0.6.0**, branch
`codex/16-20-delivery-5`, based on integrated Delivery 4 at `fc072bc`.
The user separately authorized Delivery 5 and agreed its initial scope.
[PR #28](https://github.com/alexCoding42/dfir-log-explorer/pull/28) contains the
implementation. This record is not human acceptance or merge authorization.

## Checks

| Check                                                                 | Actual result                                                                                                                                                                                          |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `cargo test -p dfir-log-core --locked`                                | Passed: 69 tests after checkpoint-boundary validation and multiline lookahead coverage.                                                                                                                |
| `cargo test -p dfir-log-core --test volume --locked`                  | Passed: 10 tests, plus one ignored subprocess-worker entry executed by its parent crash-recovery test.                                                                                                 |
| `cargo clippy -p dfir-log-core --all-targets --locked -- -D warnings` | Passed after the final test addition.                                                                                                                                                                  |
| `cargo fmt --all --check`                                             | Passed.                                                                                                                                                                                                |
| `npm run build`, `npm test`                                           | Passed: frontend build and 6 tests, including the new component workflow.                                                                                                                              |
| Chromium, existing suite                                              | Passed: all 18 workflows, one worker and 60-second per-test timeout in the shared container.                                                                                                           |
| Chromium, Delivery 5 command boundary                                 | Passed: checkpoint resume, JSON Lines/CSV selection, raw consent, export progress and no horizontal overflow at 1000 × 720. Uses synthetic Tauri IPC; it is not native filesystem validation.          |
| `npm run format:check`, `git diff --check`                            | Passed after formatting the generated measurement JSON.                                                                                                                                                |
| Native local `cargo check -p dfir-log-explorer --locked`              | Blocked: `glib-sys` could not find system `glib-2.0 >= 2.70` through pkg-config. Native application checks/builds must be verified in platform CI; no local native executable or installer is claimed. |
| Linux/Windows release tests and packages                              | GitHub CI started from PR #28. Consult its current checks; running/queued jobs are not successful checks. No human Windows acceptance or new offline-traffic audit was performed locally.              |

The Rust coverage includes gzip member concatenation, CRC/truncation failures,
a small injected decompression bound, invalid UTF-8/CRLF equivalence, ordinary
rollback, invisible staging, save/reopen/resume, a child process exiting without
Rust destructors, source digest changes, invalid checkpoint versions, discarded
staging, historical custom-profile retention, scalar attribute querying,
multiline lookahead boundaries, full exports beyond the visible page, quoting,
Unicode, destination collisions, cancellation, and cache invalidation on
ordinary and checkpoint publication.

Initial checks exposed an ambiguous integer type in export progress and a
legacy signature-test array type after gzip became supported; both were fixed.
The initial Cargo invocation also predated the new dependency manifest, so the
current manifest/lockfile was rechecked. The first browser run began while
Chromium was still being installed and had launch failures; the completed
installation and writable cache were used for the successful full rerun. Initial
measurements competing with compilation/tests were replaced by the retained
comparative runs. These failures are not counted as passing checks.

## Measurement method

Rust **1.99.0**, bundled DuckDB crate **1.10506.0**, debug profile with debug
symbols disabled; Linux x86_64, glibc 2.41, Intel Xeon Platinum 8573C host, a
**2-CPU quota and 8 GiB container memory limit**. DuckDB uses two threads and a
512 MB buffer budget, which is not a whole-process RAM cap. The container is
shared infrastructure; host scheduling and OS cache state are uncontrolled.

The baseline executable was built from `96a5bbe` in a detached worktree. The
final executable uses the engine implementation in `928d36b`; later changes strengthen pending-checkpoint validation and add
test/documentation/reporting content, without changing the measured ordinary-import
or query paths. Both use exactly the same benchmark
and generated Apache input: 100,000 events (10,795,000 bytes) and 1,000,000 events
(107,950,000 bytes). All events must parse, warnings must be zero, the selective
route must match exactly 1%, and first/final/deep filtered pages assert expected
IDs and counts. Input SHA-256, decoded sizes, parsed/warning counters and saved
case sizes agree before/after.

Each retained workload starts a fresh process. Input is generated immediately
before import; OS caches are not cleared. The first search includes statistics
calculation, while the following four reuse statistics and, after optimization,
the page cache. Import/query timings exclude generation and saving. Peak RSS
covers the complete child process, including generation and case saving, and
excludes the webview. These are single comparative samples, not statistical
confidence intervals or release-mode throughput guarantees. The highly repetitive
input compresses well and does not represent diverse production log entropy,
all readers, malformed records or multi-gigabyte corpora.

Reproduce using the same build profile and machine, with no competing checks:

```sh
cargo build -p dfir-log-core --example benchmark --locked
python scripts/measure_engine.py target/debug/examples/benchmark 100000 result-100k.json
python scripts/measure_engine.py target/debug/examples/benchmark 1000000 result-1m.json
```

Use a separate worktree at `96a5bbe` and retain its executable for baseline runs.
The Python runner uses Unix resource accounting and Linux cgroup metadata.
For release measurements rebuild both revisions with `--release`; the debug
numbers below cannot be compared with those builds as equivalent workloads.

## Recorded comparison

| Workload    | Import seconds before → after | First search ms before → after | Median of following 4 identical searches ms | Deep filtered page ms before → after | Peak process RSS KiB before → after |
| ----------- | ----------------------------- | ------------------------------ | ------------------------------------------- | ------------------------------------ | ----------------------------------- |
| 100k events | 6.734 → 7.120                 | 176 → 185                      | 42 → <1                                     | 44 → 52                              | 135,248 → 135,824                   |
| 1m events   | 70.847 → 68.922               | 456 → 664                      | 79 → <1                                     | 186 → 237                            | 286,776 → 266,620                   |

The selected optimization targets the measured repeated-page scan: retain at
most four complete preview pages, keyed by the full validated query, and clear
them after every source publication. The engineering comparison target was at
least 75% lower repeated-page latency on this benchmark; observed cached calls
are below the timer's 1 ms resolution. This is not zero-cost retrieval or a
product performance guarantee. Eviction, new queries and uncached offsets still
execute SQL. The cache adds bounded preview/statistics copies and no new
persistent index or database columns. Its whole-process memory effect cannot
be isolated from these RSS samples.

The 100k import is approximately 5.7% slower in the retained sample. First
search and uncached deep-page times also regress in both rows; no generalized
query-speed improvement is claimed. Ingestion dominates total wall time, and
uncached statistics/filtering/deep pagination remain the next measurement
priorities. Larger/diverse corpora, repeated release-mode samples and UI/native
measurements are required before choosing another optimization or promising
throughput targets. The present change preserves result semantics; it does not
solve every large-volume bottleneck.

Raw records:
[baseline 100k](measurements/delivery-5-baseline-100k.json),
[baseline 1m](measurements/delivery-5-baseline-1m.json),
[final 100k](measurements/delivery-5-final-100k.json),
[final 1m](measurements/delivery-5-final-1m.json).

## Review handover

Issues #16–#20 are implemented for review, not declared Done. Use the
[acceptance guide](delivery-5-acceptance.md) for exact storage, recovery,
compression, export and compatibility boundaries. Native Linux/Windows CI,
interactive platform acceptance and separate human delivery acceptance/merge
authorization remain outstanding. PR checks and the linked records must retain
actual results rather than treating implementation or passing engine checks as
acceptance.
