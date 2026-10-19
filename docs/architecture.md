# Architecture

## Decision

Use a modular desktop application: Tauri 2 hosts React/TypeScript, Rust owns ingestion and analysis, and an embedded DuckDB database stores each local case. There is no production HTTP server or network data plane.

The UI and core are separated so parsing, integrity handling, and queries can be tested without a graphical desktop. The repository uses a Cargo workspace with `dfir-log-core` as the default member and the Tauri application in `src-tauri`.

## Data flow

```text
OS file picker
  -> selected path held in Rust
  -> read-only buffered reader
  -> bounded line decoder + SHA-256
  -> bounded native ModSecurity framing when explicitly selected
  -> Apache / Nginx / JSON Lines / Syslog / iptables / HAProxy / ModSecurity / text parser
  -> transactional DuckDB Appender
  -> bounded query result / selected event detail
  -> React virtualized table
```

The webview supplies a parser choice, search text, pagination, and an event ID. It cannot supply an arbitrary import path; the backend imports the path selected through the native dialog. Filesystem, shell, updater, and network plugins are not exposed to the frontend. The dialog plugin is called from Rust.

## Ingestion

One worker owns database access through a mutex. Blocking work is moved to Tauri's blocking runtime rather than the UI thread. A separate atomic cancellation flag is readable without acquiring the database mutex.

The reader buffers 256 KiB and caps individual lines at 1 MiB, including the terminator. Explicit ModSecurity mode also caps aggregate records at 1 MiB and checks cancellation between physical lines. A fresh A boundary resynchronizes an incomplete transaction; only a matching Z closes it. Other modes retain one event per physical line. DuckDB's Appender buffers and flushes rows in bulk. Progress is throttled to approximately ten updates per second and sent over a Tauri channel. It reports bytes read, total source bytes, line count, and elapsed time.

Each import appends one source and its events/JSON scalar attributes/summary inside a transaction. The appender is explicitly flushed; cancellation and errors roll back. Every previous source remains available after rollback. Queries are deliberately suspended during import in this MVP. Partial results, resumable ingestion, and concurrent exploration will require a separate storage/transaction decision.

Source bytes are hashed as they are consumed. File size and modification time are checked at the end. This detects common concurrent changes, but cannot detect every possible same-size mutation or establish forensic custody.

## Event model

Every record retains its source line, byte offset, original byte sequence, decoded display text, parser name, mutually exclusive parsing status, fields, and optional warning. Source summaries additionally retain the selected parser and local path/provenance. Cases use explicit schema version 1. Unknown versions are rejected before switching the active case; no migration is implemented. Event IDs are global and contiguous across all committed sources; source IDs are stable, while line numbers and byte offsets restart for each source. Event IDs count logical events; multiline provenance uses the first physical line/byte offset, detail computes the final line from retained bytes, and source summaries additionally count physical lines. No database column migration is needed. HTTP fields include the original-offset ISO timestamp, UTC epoch milliseconds, remote host/address, request method/path/protocol, status, response bytes, referrer, and user agent.

Unknown lines in automatic mode are plain text. A matching Apache record with invalid fields is Partial with an engine diagnostic; independent validation preserves all valid fields. Pattern mismatches are Unparsed. Explicit plain-text mode considers successfully read lines Parsed without claiming structured extraction. Explicit Apache mode also marks unmatched lines as warnings. Common/Combined optional fields and the Apache `-` sentinels remain valid absences. Encoding warnings are independent of parsing status. Original Apache escapes are preserved rather than interpreted. Blank lines are records. A final line without a newline is retained.

The MVP 0 timestamp representation preserves the source offset and a numeric instant. JSON Lines uses the same offset-preserving text/UTC millisecond model. The four explicit security readers follow the [MVP 2 format contract](mvp-2-formats.md); ambiguous BSD/HAProxy dates remain attributes with diagnostics, never invented instants. Original JSON objects and security-reader attributes remain available in the inspector; top-level scalars are stored separately for parameterized exact filters and decoded-value search. Ambiguous dates never receive an implicit local timezone. See [format rules](mvp-1-formats.md).

## Query and UI boundaries

Queries use bound parameters. User text is a literal substring, not SQL or a regular expression. Optional backward-compatible query fields select the search scope, parsing status, exact field value and UTC range (inclusive start, exclusive end). All search/sort columns are whitelisted Rust enums; values use bound parameters. Global search covers raw text, normalized fields and decoded JSON scalar values. Cases without JSON scalar attributes retain the simpler raw/timestamp predicate; successful appends and reopened cases establish whether the JSON search path is needed. JSON escapes mean extracted strings may differ from their original raw representation. Result pages are limited to 500 events. Raw list previews, URLs, referrers, and agents are limited to 240 characters, with smaller bounds for other extracted list strings. Full display text and complete extracted fields are returned only for the selected event. This prevents unusually long HTTP fields from inflating an entire IPC page. The table virtualizes the page; it does not accumulate the whole dataset in browser memory.

Pages default to event-ID order. Sorting is executed across the complete filtered dataset in DuckDB, with nulls last and source ID as a deterministic tie-breaker. In an unfiltered session, IDs are contiguous because all source lines are retained, so an offset maps directly to an ID boundary. Those pages use `id > boundary` instead of sorting/skipping earlier events; the total uses a simple table count. Filtered searches/counts scan text, and deep filtered offset pagination may become slower. Further optimization must be driven by measured workloads, including filtered cursor navigation and a dedicated text index if needed.

Each query also returns exact aggregate statistics for all matches, not just the visible page. Up to 32 contiguous time buckets contain parsing counts and HTTP response distributions. Missing timestamps are counted separately. The chart explicitly uses UTC; inspector/table timestamps retain source offsets. Aggregate responses for up to four recent predicates are cached in memory independently of sort and pagination. Successful imports invalidate the cache; rollback preserves it.

The frontend separates workspace chrome, source header, parsing summary/status badges, filters, timeline, virtualized event table, tabbed inspector and secondary views. Explore gives the available height to the table; case actions and advanced filters open on demand, while Statistics contains the existing aggregates and timeline. Default columns are line, timestamp, parsing status and raw preview; a source column is mandatory in multiple-source cases. Legacy default column preferences migrate to this compact layout, while customized layouts remain intact. Light/Dark/System, sidebar width and visible columns are the only localStorage preferences; evidence and searches are not persisted there.

Source import and query response types are serialized with camelCase keys. The generated synthetic preview fixture exercises the same serialization as production.

## Storage and runtime security

Each unsaved engine creates a unique temporary directory and disk-backed database. Save checkpoints and closes the current connection (required for Windows file locking), copies the complete database into a same-directory temporary file, syncs it, then publishes it with no overwrite. The current connection is restored even after a copy error, and the engine switches to the saved path only after reopening/validation succeeds. Persistent sources and summaries are committed with their events; normal closure retains saved cases. Native save/open dialogs alone choose paths, and all case mutations share the import operation guard. A new case starts temporary; the UI requires saving populated temporary cases before switching. DuckDB WAL files can exist while saved cases are open, so cases should be closed before copying/backing up. DuckDB external access, automatic extension installation/loading, and community extensions are disabled, then settings are locked. The 512 MB budget applies to DuckDB's buffer manager, not to all native allocations. Two DuckDB worker threads are used initially to avoid aggressive CPU/memory contention on small machines.

The production CSP permits only local application assets and Tauri IPC. Log content is rendered through React text nodes; it cannot inject HTML or execute scripts. Development HMR uses localhost only and has a separate CSP.

Temporary storage is explicitly dropped in Tauri's exit callback, because the framework can terminate without normal stack unwinding. The callback signals import cancellation before closing the database. Crash leftovers are possible; there is no secure erasure or application-level encryption in this MVP. See the privacy document.

## Distribution

Build the Windows and Linux installers on their respective platforms. Initial support is Windows 11 x64 and Ubuntu 24.04 x64. WebView2 and WebKitGTK differ, so a successful compile does not substitute for interactive acceptance testing on each platform.

Windows MSVC builds use a static C/C++ runtime, consistently across direct Cargo checks and Tauri packaging. The `cc` build driver propagates `crt-static` to the embedded DuckDB compilation. See the build-verification document for platform checks and artifact availability.

The initial Windows package embeds the offline WebView2 installer. This is an explicit MVP 0 packaging compromise compared with the proposed fixed runtime: users get offline installation, while the shared runtime lifecycle remains outside application control. Fixed-runtime distribution will require bundling, licensing review, and a runtime patching process.
