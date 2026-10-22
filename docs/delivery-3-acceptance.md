# Delivery 3 acceptance guide

Delivery 3 version 0.4.0 extends the accepted Delivery 2 workspace. The user authorized development on October 19, 2026 after reviewing the three-part scope. Issues [#9](https://github.com/alexCoding42/dfir-log-explorer/issues/9), [#10](https://github.com/alexCoding42/dfir-log-explorer/issues/10) and [#11](https://github.com/alexCoding42/dfir-log-explorer/issues/11) hold decisions, actual verification, PR/CI links and handovers. This guide defines review scenarios; unchecked human acceptance is not inferred from automated checks.

## Timeline investigation

Open Statistics after importing the synthetic sample or a case with dated events. Existing interval clicks still open the corresponding logs. Choose **Interval action → Zoom in** to refine the histogram while staying in Statistics. **Previous time range** restores the previous bounds; **Reset time range** removes only the time bounds. Search, source, reader, parsing, warning and exact filters remain active.

UTC ranges have an inclusive start and exclusive end. Source offsets and raw evidence are unchanged. Histograms contain at most 32 buckets with a minimum interval of one second; history retains the most recent 20 zoom levels and resets when a range is set manually or a saved search is applied. The chart represents matching dated events, not fabricated timestamps or a full acquisition chronology. Repeated zoom into a one-second interval provides no finer resolution. Undated events remain available when no time bound or timestamp-presence filter excludes them. Empty or entirely undated results display explicit explanations.

Human checks:

- [ ] Apply a source and literal search, zoom into an interval, return to the previous range and reset. Non-time filters remain unchanged.
- [ ] Choose View logs and select an interval. Every listed event satisfies the active filters and half-open UTC bounds.
- [ ] Check empty results, only undated events and keyboard access to intervals and navigation.

## Investigation breakdowns

The new breakdowns answer: which client IPs and request paths occur most often, and which sources and readers contribute to the current results? Each dimension shows up to ten values, ordered by event count descending and full value ascending for ties. Counts cover all matching events, including rows beyond the current page. These counts are event frequency, not inferred attacks, request rate, correlations or deduplicated requests. Existing HTTP counters and source-total parsing cards retain their documented scope.

Select an IP or path to append an exact investigation filter, or select a source/reader to refine that scope. Existing exact-field and scalar-attribute filters remain active. Investigation filter chips can be removed individually; Reset clears all filters. Counts for missing IP/path values are shown separately. Undated events contribute unless excluded by time-related filters. Source names are display labels; identity uses the source ID. Missing field means null; an extracted empty string remains a separate value.

Labels display at most 240 characters. Exact drill-down uses the complete value up to 4096 characters; longer values are visible as bounded labels with their drill-down disabled. At most 16 additional exact investigation filters can be combined. A hidden remainder is possible with more than ten distinct values; top rows are not a complete partition of the result set.

All aggregates are calculated inside DuckDB over the complete predicate, with a four-predicate cache reused across sorting and pagination and invalidated on successful import. Grouping may scan the matching dataset and its cost depends on cardinality. The four grouped queries and two missing-field counts add work on uncached predicates. The 512 MB DuckDB buffer budget is not a process-wide memory limit. This delivery does not claim representative large-volume performance results.

Human checks:

- [ ] Compare the displayed counts with known synthetic events, including a case with multiple sources and fewer visible rows than matching events.
- [ ] Drill into an IP and then a path while retaining a search and another exact filter. The event list matches the displayed count and all active constraints.
- [ ] Check missing fields, ties, empty results and unusually long values.

## Saved searches

In Explore, open **Saved searches**. Enter a name and choose **Save as new** to capture the current query. Use **Find a saved search** to filter the list. Choose a definition and **Apply search** to restore it. **Update selected** replaces its name and query with the currently active values; to rename while preserving its query, apply it first and then update the name. **Delete search** opens an inline confirmation and affects only that definition.

Saved parameters include literal search and selected search field, source ID, identified reader, timestamp presence, parsing status, warning flag, exact field and added investigation filters, exact scalar JSON/reader attribute filter, absolute UTC bounds, sort field and direction. Pagination restarts at the first page. Theme, columns, selected event, chart presentation and zoom history are not search parameters. Searches are query definitions, not frozen event snapshots: later imports may change results for an all-source search.

Names are trimmed, nonempty, unique without case sensitivity and at most 120 Unicode characters. Each case allows 200 definitions; a serialized definition is limited to 128 KiB, with an explicit error if its combined filters exceed that limit. An unavailable referenced source is an error; applying the definition leaves the active filters unchanged. Unsupported versions, malformed definitions and unknown query parameters are rejected rather than silently ignored.

Searches belong to the active case. They can be created in a temporary case and are copied with it when saved. Closing an unsaved temporary case through normal application exit loses its searches along with that case; save it first. In an existing saved case, successful edits commit locally. Save/open/new operations reset the workspace filters and reload the active case's definitions. No search or evidence content is written to browser preferences. The browser synthetic preview demonstrates search management in memory only; closing/reloading the page loses it.

### Persistence and compatibility

The evidence schema remains version 1. Search definitions use an optional `saved_searches` table with numeric identity and a JSON payload carrying independent definition version 1. Opening/listing an old case without this table does not modify or migrate it. The table is created transactionally only on the first successful search write. Raw events, attributes and source metadata are never changed by search creation, update or deletion. Older application versions can still read the underlying evidence but do not expose saved searches. Cases remain unencrypted local files; search text is sensitive case content and inherits the existing case handling rules.

Human checks:

- [ ] Save a named query combining filters and sort order, save the case, create a new case, reopen the saved case and apply the query. Compare with the equivalent manual query.
- [ ] Update a query, rename it, cancel a deletion, then confirm deletion. Verify raw content and provenance remain unchanged.
- [ ] Open a Delivery 1/2 case without searches; read it and then save its first search. Verify evidence compatibility.
- [ ] Check duplicate/empty names, unavailable sources and unsupported definition errors. Check that queries do not leak between cases or into browser preferences.

## Application version

Open **Settings → About** to read the application name and version, then use **Copy version** when reporting an issue. The version comes from the packaged frontend's package metadata. It remains accessible with no source imported and while footer notifications are displayed. The footer shows local case status and notifications; version information is available in Settings.

- [ ] Confirm that Settings → About displays version 0.4.0 and Copy version copies `DFIR Log Explorer 0.4.0`.
- [ ] Check the version before import and after a notification appears; dismissing a footer notification does not reveal another version label.

## Review boundary

Automated results are recorded separately in [the validation record](delivery-3-validation.md). Interactive Windows acceptance and a fresh native offline-traffic audit are distinct checks. No parser profiles, new readers, annotations, correlations, exports, compression or resumable ingestion are included. Human acceptance and explicit merge authorization are required before integration; Delivery 4 requires a separate start instruction.
