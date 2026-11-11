# Delivery 4 — Local parsing profiles

Review version **0.5.0**. The user authorized Delivery 4 on November 11, 2026 and selected an application log in `date | level | message` form as its initial acceptance format. Issues [#12](https://github.com/alexCoding42/dfir-log-explorer/issues/12), [#13](https://github.com/alexCoding42/dfir-log-explorer/issues/13), [#14](https://github.com/alexCoding42/dfir-log-explorer/issues/14) and [#15](https://github.com/alexCoding42/dfir-log-explorer/issues/15) preserve the agreed contract, live status and handover. Human acceptance and explicit merge authorization remain separate requirements; Delivery 5 is not authorized.

## Create, preview and use a profile

1. Open **Parsers → Local parsing profiles** in the desktop application. Start a new profile or select an existing profile and a revision from **Version history**.
2. The starter pattern is `(?P<timestamp>[^|]+?) \| (?P<level>[A-Z]+) \| (?P<message>.*)`. It describes a single physical line, such as `2026-11-11T12:00:00Z | INFO | Server ready`.
3. Paste a small synthetic sample into **Sample text** and choose **Preview profile**. Check each record's raw text, physical line/byte position, extracted attributes, timestamp, parsing status and diagnostic. An empty sample is valid and returns zero records. Preview never imports evidence or changes the profile.
4. Choose **Save as new profile**. To edit a profile, load its latest revision, change its name or pattern, then choose **Save new revision**. An earlier revision is immutable. A stale editor is rejected rather than silently overwriting newer work. A historical revision can still be previewed or selected for import.
5. Choose **Use revision N for next import**, then **Import log file** and select `tests/fixtures/application.log`. The import selector identifies the saved profile and revision. Editing an unsaved draft requires saving or reloading it before selecting a revision. Previewing a draft does not implicitly save it or change the next import.
6. Inspect the four imported events: two Parsed, one Partial (invalid timestamp), one Unparsed (pattern mismatch). Inspect their raw content, source positions, timestamps and reader attributes. Filter **Reader → Custom profile**, or filter an attribute such as `level = ERROR`.
7. Open **Evidence** to inspect the source's profile name, revision, pattern and definition SHA-256 alongside the source SHA-256. Events reference their source; its full profile definition is retained even after later profile edits.
8. Save the temporary case, close it and reopen it. Confirm that history, source definitions, original evidence and query results remain available. Create another revision and import the same file with it; the prior import must keep its original interpretation. Explicitly selecting a historical revision must use that revision.

Profiles live inside the current case. Temporary profiles are lost on application exit unless the case is saved. The UI requires saving a temporary case containing profiles before switching cases, including when it has no sources. Profiles do not transfer to a different case; opening/new-case actions clear the next-import profile selection. There is no profile deletion, profile export/import or machine-wide profile library in this delivery. A saved case is the transfer unit.

## Profile definition and field rules

Definition version 1 contains only `version`, `name` and `pattern`; unknown configuration and unsupported versions are rejected. A stored revision adds `id`, `revision` and a SHA-256 over the UTF-8 JSON definition serialized in `version`, `name`, `pattern` field order. The digest identifies definition content; it is not a digital signature or a guarantee of correct interpretation.

- Name: 1–120 Unicode characters after trimming, without control characters. Another profile's historical name cannot be reused (case-insensitive comparison). Edits can rename the same profile.
- Pattern: 1–4096 UTF-8 bytes. The Rust `regex` engine wraps it in absolute full-record boundaries. Partial substring matches are not accepted.
- Captures: 1–32 named captures; ASCII letters/digits/underscore names up to 32 bytes. Every capturing group must be named; use `(?:...)` for noncapturing groups. Duplicate or invalid names are rejected by the engine.
- Every present capture remains a string in reader attributes. Optional unmatched captures are absent. Other capture names such as `level` and `message` are permitted without a fixed application-specific schema. The event's raw display text remains the complete original record rather than the `message` capture.
- `timestamp`: RFC 3339 with an explicit offset, within the application's supported date range. The original string remains an attribute; the normalized timestamp retains its offset and supplies UTC milliseconds for filtering and statistics. No local timezone or epoch unit is guessed.
- `source_ip`, `method`, `path`, `protocol`, `referrer`, `user_agent`: populate the corresponding existing string fields. As with existing access readers, an address string is not independently validated as an IP address.
- `status`: decimal HTTP status 100–599. `response_bytes`: nonnegative signed 64-bit decimal integer. Invalid normalized numbers/timestamps remain in attributes and produce **Partial** with field diagnostics. Valid independent fields are retained.
- No full-line match: **Unparsed**, with a profile mismatch diagnostic and original text retained. Successful full match with no conversion errors: **Parsed**. Missing optional captures do not automatically cause a failure. Parsed does not mean evidentially verified.

Profiles cannot execute scripts, load remote code, use regex backreferences/lookaround or change multiline framing. Rust regex avoids backtracking-style catastrophic execution. Pattern nesting is limited to 32; compiled regex and DFA cache budgets are each 1 MiB. These are regex budgets, not total-process RAM limits or elapsed-time guarantees. Imports compile the selected rule once and reuse the existing cancellable, disk-backed ingestion path.

## Resource and compatibility boundaries

Preview rejects samples above **64 KiB of UTF-8 text or 100 physical lines** instead of silently truncating. A final newline does not create an extra empty record; an actual blank line does. CRLF and byte offsets follow the import framing rules. The preview uses the exact Rust profile reader on a background worker. Its raw display includes the sample's line terminators; the inspector's display omits terminators as before, while stored raw bytes retain them. Invalid source UTF-8 still produces replacement-character display and the existing encoding warning during import; a pasted UTF-8 sample cannot establish equivalence for invalid original bytes.

Each case permits **50 profiles**, **20 revisions per profile**, **200 revisions total** and **16 KiB per serialized stored revision**. Reaching a limit rejects a save and preserves history. Storage is an optional `parsing_profiles` table, created transactionally on first save alongside evidence schema 1. Opening/listing an old case never creates it. Unknown/malformed definitions, mismatched digests/identifiers, missing versions and incompatible extension storage are explicit errors; the application does not automatically migrate or repair them. Previously accepted evidence remains on disk.

A source imported with a profile retains the complete chosen revision, including definition and digest, in its source summary. Events retain source IDs, raw bytes, physical lines, offsets, extracted fields and diagnostics. Editing profiles never reparses existing events. Older application builds that do not recognize custom readers cannot open cases containing these new sources; use version 0.5.0 or later for those cases. No downgrade migration is provided. Existing cases with native readers remain supported.

The existing 1 MiB record limit, source-stability checks, source SHA-256, read-only file access, transactional cancellation/error rollback, query limits and local operation still apply. Cases/profile definitions are unencrypted, and a pasted preview may include sensitive data; keep test/review examples synthetic. The browser preview exposes the profile layout and desktop requirement but cannot save, interpret or import custom profiles; it does not implement a second parser.

## Human acceptance checklist

- [ ] Create a profile, diagnose an invalid pattern and inspect Parsed/Partial/Unparsed preview rows with the agreed synthetic format.
- [ ] Import using the selected saved revision and compare normalized fields, attributes, diagnostics and source ranges with preview.
- [ ] Create a new revision, retain earlier source definitions/interpretations, and explicitly import with a historical revision.
- [ ] Save/reopen a case and verify history, definition digests and original evidence. Check protection of an unsaved profiles-only case.
- [ ] Confirm preview bounds and errors, legacy-case behavior, rollback/cancellation, and native-reader/saved-search regressions from the recorded test evidence.
- [ ] Exercise Windows interactively and review the CI results, performance boundaries and unexecuted checks before acceptance.

Actual automated/local results belong in [the validation record](delivery-4-validation.md) and linked Issue/PR handovers. This checklist does not claim human acceptance.
