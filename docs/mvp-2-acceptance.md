# MVP 2 acceptance — v0.3.0

MVP 2 development was requested on 2026-10-15 after verifying that `main` at `bb51d382eafcbaf642f66818a69b933a7df3dd0b` includes merged PRs #3 (MVP 1), #4 (README) and #5 (Statistics). No pending PR required integration. This build requires separate human acceptance before MVP 3 begins; its PR is not merged automatically.

## Scope

Four explicit readers: Syslog RFC 5424/BSD file layout, Netfilter/iptables LOG, HAProxy HTTP, and multiline native ModSecurity 2.x audit transactions. The [format contract](mvp-2-formats.md) specifies supported variants, dates, diagnostics, framing limits and exclusions. This is not universal firewall/WAF format support.

## Walkthrough

Build the desktop app as described in the README, or download the Linux/Windows artifact from the MVP 2 workflow for this branch. Browser preview retains the existing Apache synthetic sample; real imports require the desktop app. Use only the synthetic fixtures below for acceptance.

1. Select **Syslog** and import `tests/fixtures/syslog.log`. Expect 5 events: 2 Parsed, 2 Partial, 1 Unparsed. Inspect the first record's host, app, facility, severity and decoded structured data. The source offset is +02:00. The BSD date remains an attribute with an ambiguity diagnostic, not an invented UTC instant.
2. Select **Netfilter / iptables** and import `iptables.log`. Expect 3 events: 2 Parsed, 1 Partial. Inspect IPv4 and IPv6 addresses and `firewall.DPT`. Verify that invalid source/port fields produce diagnostics without removing the destination port. The firewall prefix is not an inferred verdict.
3. Select **HAProxy HTTP** and import `haproxy.log`. Expect 3 events: 1 Parsed, 2 Partial. The explicit-offset first request has status 200 and 512 response bytes. The standard date on the second request has no normalized instant. The third is a logasap lower bound with no exact response-byte value or HTTP status.
4. Select **ModSecurity audit** and import `modsecurity-audit.log`. Expect 3 events from 30 physical source lines: 2 Parsed, 1 Partial. Event 1 spans lines 1–19; event 2 spans lines 20–26; event 3 spans lines 27–30. Inspect the complete raw transaction, two WAF messages, action and request/response fields. The final incomplete transaction retains `/incomplete` and reports both timestamp and missing-Z diagnostics. WAF alerts themselves are not parsing warnings.
5. With all four sources imported, expect **14 events**. Open Filters. Combine source/reader and Attribute / JSON field `firewall.DPT` = `443`: with the firewall source selected, expect 2 records; with Valid timestamp as well, expect 1. Inspector attribute filter buttons should apply the same exact filter.
6. Inspect a Syslog structured-data value containing decoded quotes/brackets. Copy it, then filter by that attribute. Confirm literal SQL-looking search/filter text has no special effect. Raw text stays inert even if it contains HTML-like content.
7. Save a new `.dfir` case, create a new case, then reopen the saved case using the native dialogs. Verify 4 sources, 14 events, source hashes/provenance, multiline ranges, attributes and combined filters. No original source is needed to inspect the saved evidence. Open an existing MVP 1 case separately and confirm it remains readable.
8. Cancel a further import and confirm previous evidence remains intact. An oversized line or multiline transaction must fail atomically with the 1 MiB diagnostic, preserving source counts and queries. The automated regressions construct those files in temporary storage.
9. Check Light and Dark mode, reader selection/help, keyboard navigation and the default compact table. Statistics must operate on all matching events, with ambiguous timestamps counted as missing rather than silently assigned a timezone.

## Acceptance record

Record the OS, artifact commit, passed/failed steps and any real-format differences. Automated fixtures validate the documented layouts; they do not establish coverage for all devices, custom logging configurations or corrupted audit streams. Interactive acceptance remains a human decision. See the [validation record](mvp-2-validation.md) for executed checks and limitations.
