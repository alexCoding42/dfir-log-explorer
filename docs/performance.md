# Performance validation

The architecture is designed for local analytical workloads. No equivalence with commercial DFIR products is claimed without comparative measurements.

## Reproducible engine workload

The benchmark generates synthetic Apache events in a temporary file, imports them through the actual engine, verifies event counts, searches a route occurring in one percent of events, and queries the final page. It never uses private logs.

```sh
cargo build -p dfir-log-core --example benchmark --release --locked
/usr/bin/time -v target/release/examples/benchmark 1000000
```

PowerShell can execute `target\release\examples\benchmark.exe 1000000`; use the platform's process-monitoring tool for peak memory.

The JSON output records profile, source bytes, import seconds, events/second, MiB/second, first page time, five literal searches, and final page time. Engine-only memory measurements exclude the UI/webview. A debug run is useful for correctness but is not a release performance claim.

The Delivery 5 preparation also records the input SHA-256, parsed/warning
counts, deep filtered pagination, saved database size and save duration. Saving
runs after query measurements, outside import/query timing. `/usr/bin/time -v`
covers the entire process, including generation and saving; its peak RSS cannot
be attributed exclusively to ingestion. The first query uses a fresh engine,
but the input has just been generated and OS caches are not cleared. Repeated
searches also reuse the engine's statistics cache. Retain these distinctions in
any before/after comparison. The [Delivery 5 plan](delivery-5-plan.md) contains
proposed workloads and outstanding decisions; no new measurements are claimed
by adding instrumentation.

## Comparison protocol

- Use the same hardware, source files, extraction requirements, and result semantics.
- Separate file generation, initial import/indexing, first query, and repeated queries.
- Record failures, warning counts, disk amplification, peak process memory, and UI responsiveness.
- Test Apache, unstructured text, unusually long lines, and malformed events.
- Begin with approximately 100 MB, then expand to 1 GB and 10 GB in later validation steps.
- For product comparisons, specify the tool/version, indexing configuration, and included preprocessing.
- Do not compare a warm indexed search with a cold unindexed scan as if both incurred the same preparation cost.

## Initial limitations

Filtered searches/counts currently scan decoded text, and deep filtered offset pagination is not guaranteed to be constant-time. Unfiltered pages use contiguous event-ID boundaries to avoid skipping earlier results. Imports are atomic and block exploration until completion. The database buffer budget is 512 MB with two threads, while total process memory can exceed that budget. Filtered cursor navigation, indexes, and incremental imports will be considered against observed bottlenecks.

Actual local verification measurements are recorded in `docs/validation.md` when available; they are not universal performance guarantees.
