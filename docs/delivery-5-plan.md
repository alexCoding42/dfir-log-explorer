# Delivery 5: implementation proposal

Started on November 13, 2026 on `codex/16-20-delivery-5`, based on
`fc072bc` (Delivery 4 integrated through PR #27). The user explicitly authorized
starting Delivery 5. The user subsequently agreed the proposed initial measurement, gzip, export and
resume scope. See [the acceptance contract](delivery-5-acceptance.md) for the
implemented boundaries. This planning history is not delivery acceptance or a
claim that benchmarks passed.

## Measurements and optimization (#16–#17)

Proposed first investigation timebox: 30 minutes of measurement and analysis,
excluding dependency installation and compilation. Start with 100,000 and
1,000,000 synthetic Apache events, then synthetic text if the first runs fit the
available disk and memory. Record the exact commit, Rust/DuckDB versions, CPU,
memory, OS, build profile, source bytes and hash, wall time, peak process RSS,
database size, warning counts and correctness assertions. Measure import,
first query, repeated literal searches, final unfiltered page and deep filtered
pagination separately. Do not claim cold OS caches: the generated input was
just written; first query and repeated queries have different cache conditions.
This is an engine baseline; UI/webview responsiveness needs separate checks.

Rank measured bottlenecks, propose a measurable target and storage/resource
trade-offs, and obtain agreement before implementing the selected optimization.
Run identical before/after workloads. No performance guarantee is agreed yet.

## Resume contract (#18)

Proposed design: persist staging events and checkpoints in a saved case, separate
from published source events. Each batch and its checkpoint commit together.
The checkpoint retains source identity/digest, complete parser/profile revision,
record boundary, physical line count, byte offset and diagnostic counters.
Resume only at a complete record boundary, including multiline audit records.
Verify the full source digest before resuming; size and mtime alone are insufficient.
Changed sources and incompatible checkpoint versions are rejected explicitly.

An interrupted import remains invisible to queries, statistics and exports.
Publish source metadata and staged events atomically only after completion and
source verification. Completion must neither lose nor duplicate events and must
invalidate statistics caches. Existing completed sources remain usable after
reopening. Offer resume and explicit discard; discard removes staging only.
Keep the existing ordinary import cancellation/rollback behavior until the user
explicitly chooses a resumable import. Temporary cases must be saved first.
Test process termination as well as cooperative pause, reopen, changed sources,
profile edits, invalid checkpoints, duplicate resume and publication failure.

Compressed input may require replaying decompression to the checkpoint; do not
promise random access or byte seeking inside gzip. Record compressed and decoded
coordinates separately. The staging design and compatibility contract need
agreement before schema or transaction changes.

## Compression (#19)

Proposed initial format: gzip, including concatenated gzip members. Other archive
formats remain explicitly unsupported. Validate trailers and reject truncated or
corrupt streams atomically. Proposed default limit: 10 GiB decoded bytes per
source, checked while streaming, with no unbounded decompression buffer. The
existing 1 MiB record bound and cancellation checks still apply. This limit is
not a whole-process RAM or disk budget.

Retain compressed file digest/size as source provenance and decoded digest/size
separately. Event offsets address the decoded stream; retained raw bytes are
decoded evidence bytes, not compressed file ranges. Show those semantics in the
inspector and export. Verify equivalence with uncompressed fixtures, including
CRLF, invalid UTF-8 and multiline records. Test resource limits and cancellation.

## Exports (#20)

Proposed formats: UTF-8 CSV and JSON Lines, for all results matching the current
filters in the selected sort order, independent of the visible page. Include full
fields, diagnostics, source identity/digests, physical record range, byte offset
and profile revision provenance. Include raw content only when explicitly chosen;
retain exact bytes using a documented hex field for invalid UTF-8. Do not export
truncated list previews as full evidence fields.

Stream bounded batches to a temporary destination in the chosen directory.
Report progress, allow cancellation, and publish without overwriting existing
files only after success. On errors/cancellation remove incomplete output and
preserve source evidence. Avoid per-row detail queries and repeated whole-case
statistics. Test quotes, newlines, Unicode, nulls, large fields and sorting ties.
CSV must document spreadsheet formula handling without silently altering
evidence values; JSON Lines remains the lossless machine-readable option.
Exports are local, unencrypted and may contain sensitive source paths, raw logs
and personal data; explain this at export selection.

## Handover and acceptance

Issues #16–#20 remain the tracking records. Feature implementation must follow
agreement on the outstanding choices above. Record actual commands, failures,
results and CI links without treating a proposal as a completed check. Prepare a
reviewable PR with an acceptance guide and limitations. Human acceptance and
separate merge authorization are still required.
