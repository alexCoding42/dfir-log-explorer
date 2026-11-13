# Delivery 5: volume, ingestion and exports

The user authorized Delivery 5 and agreed its first scope on November 13, 2026:
synthetic 100,000/1,000,000-event engine measurements, gzip with an explicit
limit, CSV/JSON Lines exports of filtered results with provenance and optional
raw evidence, and integrity-checked resumable ingestion with hidden partial
results. Branch: `codex/16-20-delivery-5`; Issues #16–#20. This is an acceptance
contract, not human acceptance or merge authorization.

## Compression and evidence

Both ordinary imports and checkpoint imports detect gzip by its signature,
including concatenated members. CRC/trailer failures, truncated input and
unsupported ZIP/binary/UTF-16 sources fail explicitly. Decoding streams into a
local temporary file and rejects more than **10 GiB of decoded bytes**. Existing
1 MiB physical-line/multiline-event bounds remain. Temporary decoding storage is
removed on normal completion/error; an OS/process failure can leave scratch
files requiring normal temporary-directory cleanup. Disk space is required for
decoded input, staging and the final case. There is no promised whole-process
RAM or disk limit.

`SourceSummary.sha256` and `totalBytes` address decoded evidence. Optional
`compression` metadata identifies gzip and retains the compressed file size and
SHA-256. Event byte offsets and physical ranges address the decoded stream, not
the gzip file. The inspector labels these meanings. Retained BLOBs preserve
decoded bytes, including terminators and invalid UTF-8. Original sources remain
unchanged. Cancellation is checked during hashing, decoding and parsing;
reported byte/line progress describes parsing, after decoding completes.

## Durable checkpoints

Save a case first. Expand **Ingestion and export**, then choose **Import with
checkpoints…** and select a source. Ordinary **Import log file** retains its
atomic rollback/cancellation behavior. One checkpoint import may be pending per
case; resume or discard it before an ordinary/new import.

A checkpoint commits every **10,000 complete events**, along with staged event
rows and scalar attributes in a single DuckDB transaction. The optional
`ingestion_checkpoint`, `ingestion_events` and `ingestion_attributes` tables do
not change completed-case schema version 1. Pending rows never enter public
`events`/`sources` or query statistics. The checkpoint records byte/line/event
positions, counters, source digest, parser mode and the immutable profile
revision. At most the uncommitted batch must be parsed again after interruption.
DuckDB WAL recovery supplies the database transaction recovery boundary.

**Cancel import** pauses checkpoint imports, rolling back the current batch;
completed checkpoints remain. Reopen the same saved case after stopping the app,
choose **Resume import…**, and select the original file at its original path.
The full decoded SHA-256 (and compressed SHA-256 when applicable) must match;
size/mtime alone never authorize resume. Stored parser/profile choices override
the current UI selection. Gzip is decoded again into scratch storage before
seeking the decoded offset; no random seeking inside a gzip stream is claimed.

Only a fully ingested, verified source is published, with all staged records,
attributes and summary in one transaction. There are no duplicate event IDs or
visible partial results. Previous completed sources are preserved. **Discard
incomplete import** removes only checkpoint/staging data; it also permits
recovery from an explicitly rejected checkpoint version. An old application
must not modify a case with a pending import; the new application rejects a
checkpoint inconsistent with completed events/sources. No downgrade migration
or concurrent case access is supported.

## Exports

Choose **JSON Lines** or **CSV**, optionally enable **Include raw evidence**, then
**Export filtered results…**. The scope is all matching events in the selected
sort order, across all pages; page offset/limit do not restrict the export.
Native dialogs select the local destination. Existing files are never
replaced. A temporary file in the destination directory is flushed and synced
before publication; cancellation/write errors leave no published partial file.
Cancellation is cooperative between rows; blocking SQL execution must finish
before the next cancellation check. Progress reports exported event counts.

JSON Lines schema version 1 has one object per event:

```json
{
  "schemaVersion": 1,
  "source": { "id": 1, "sha256": "…", "provenance": "…" },
  "event": {
    "id": 1,
    "sourceId": 1,
    "lineNumber": 1,
    "endLineNumber": 1,
    "byteOffset": 0,
    "parser": "text",
    "parseStatus": "unparsed"
  }
}
```

Actual `source` includes its full summary, optional compression metadata and
profile revision; `event` includes full extracted fields, scalar/object
attributes and diagnostics. Truncated list previews are omitted. Optional
`raw` preserves decoded terminators; invalid UTF-8 is displayed with replacement
characters and exact bytes are retained in `rawHex`. Both fields are omitted
unless raw inclusion is selected.

CSV uses three columns: `schema_version,source_json,event_json`. The latter two
contain the same complete JSON objects with RFC-style double-quote escaping;
records end in CRLF. JSON cells preserve nested attributes, nulls, Unicode,
large fields and diagnostics without inventing flattened column names. Cells
start with an object brace rather than an untrusted evidence formula; values
inside JSON are not rewritten. This format is intended for structured analysis;
JSON Lines is usually more convenient for machines.

Exports/cases are unencrypted. Source paths, profile definitions, personal data
and optional raw evidence can be sensitive. The export controls explain this
before writing; the application neither uploads nor shares these files.

## Human review scenarios

- Import synthetic plain and gzip equivalents, including multiple members,
  malformed UTF-8/CRLF and a ModSecurity transaction; compare extracted fields,
  diagnostics, hashes, decoded byte offsets and raw details. Corrupt a gzip
  trailer and verify no source was published.
- Save a case containing a completed source, start a checkpoint import over
  10,000 events and cancel after a checkpoint. Reopen, verify the previous
  source remains and the incomplete source is hidden. Resume using the same
  input, then compare counts, raw data, attributes and provenance with an
  uninterrupted import. Change the input and verify explicit rejection.
- Export a filter matching over 500 events while viewing a later page. Check
  all matches, sort order, full long fields and source/profile provenance;
  verify raw is absent by default and exact bytes are available when selected.
  Test both formats, cancellation, an existing destination and a write error.
- Review the measurement report's hardware/profile/cache limitations and
  before/after correctness evidence. Engine runs do not establish UI, Windows
  or commercial-product performance equivalence.

Actual automated/native checks and outstanding limitations belong in the
validation record. Human acceptance and separate merge authorization remain
required.
