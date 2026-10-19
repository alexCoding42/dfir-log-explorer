# MVP 2 format contract

Version 0.3.0 adds four **explicit** readers. Select a reader before importing. Auto retains its MVP 1 Apache-compatible/JSON Lines recognition and never groups multiline records. Unsupported records remain searchable with Unparsed status; recognized records with validation problems are Partial and retain independently valid fields. A security alert or HTTP error is not a parsing warning.

All fixtures in `tests/fixtures/` use synthetic records, documentation IP ranges and reserved example domains. These readers cover the layouts below, not every product configuration.

## Syslog

**RFC 5424 version 1:** `<PRI>1 TIMESTAMP HOSTNAME APP-NAME PROCID MSGID STRUCTURED-DATA [MSG]`, one message per physical source line. PRI must be 0–191. Header bounds are validated. NILVALUE `-` is accepted for timestamp, headers and structured data. Timestamps require the RFC syntax, a known explicit offset, seconds 00–59 and at most six fractional digits. Unknown `-00:00`, invalid dates and unsupported versions produce diagnostics without discarding the payload.

Attributes include `syslog.host`, `syslog.app`, `syslog.pid`, `syslog.msgid`, `syslog.timestamp`, numeric `syslog.facility`/`syslog.severity`, and `syslog.message`. NILVALUE headers are omitted. Structured-data parameters become `syslog.sd.<SD-ID>.<PARAM-NAME>` scalar attributes. Escaped quotes, backslashes and closing brackets are decoded; malformed elements and duplicate IDs/parameters are diagnosed. Duplicate parameters and flattened attribute-name collisions are diagnosed and use the last value. The original spelling remains in raw content. An optional message BOM is removed only in the extracted attribute.

**BSD file layout:** optional `<PRI>`, `Mmm dd HH:mm:ss HOST TAG[PID]: MSG`. PID is optional. Month/day/time is validated, including February 29 as a possible date; the source has no year. A diagnostic explicitly reports the missing year and timezone. The source date remains `syslog.timestamp`; normalized timestamp and UTC milliseconds remain absent. No current year, host timezone or import-time value is substituted. This is a selected RFC 3164-style file layout, not every relay variant or TAG convention.

Transport framing (RFC 6587 octet counts), RFC 5424 versions other than 1, embedded message newlines and continuation-line heuristics are outside scope.

Primary references: [RFC 5424](https://datatracker.ietf.org/doc/html/rfc5424), [RFC 3164](https://datatracker.ietf.org/doc/html/rfc3164).

## Firewall: Netfilter / iptables LOG

Reads whitespace-separated uppercase `KEY=value` fields with required `IN`, `OUT`, `SRC`, `DST` and `PROTO`. Input/output interfaces may be empty. Bare payloads and the supported Syslog envelopes are accepted. Valid IPv4/IPv6 source addresses populate Client IP; protocol populates Protocol. Destination, ports, interfaces and other uppercase fields are available as `firewall.SRC`, `firewall.DST`, `firewall.SPT`, `firewall.DPT`, `firewall.IN`, `firewall.OUT`, `firewall.PROTO`, etc. Raw field values are retained even if validation fails.

Addresses and available ports (0–65535) are validated independently; LEN/TTL/ID/WINDOW must be nonnegative integers in the supported range. Duplicate keys are diagnosed and use the last value. The prefix preceding the first key is retained as `firewall.prefix`. Prefixes such as DROP do not establish a verdict; the reader never infers accept/drop semantics. Flag-only tokens (SYN, ACK, DF, etc.) remain in raw content. Packet LEN is not an HTTP response-byte count. Bare payloads contain no normalized timestamp; supported envelope dates follow the Syslog rules.

FortiGate, Palo Alto, pfSense filterlog, nftables custom formats, binary NFLOG and nonstandard quoting are outside scope.

Primary reference: [Netfilter logging presentation](https://workshop.netfilter.org/2015/wiki/images/5/51/Netfilter-logging.pdf).

## HAProxy HTTP

Reads the standard `option httplog` field order: client address/port, bracketed request date, frontend, backend/server, five timers, HTTP status, bytes, captured cookies, termination, five connection counters, two queue counters, zero to two captured-header blocks, and quoted HTTP request. Bare payloads and supported Syslog envelopes are accepted. IPv4, IPv6 and bracketed IPv6 clients are supported; UNIX socket clients are outside scope.

Client IP, request method/path/protocol, valid HTTP status and exact nonnegative bytes populate common fields. Other values become `haproxy.port`, `.time`, `.frontend`, `.backend`, `.server`, `.timers`, `.status`, `.request_cookie`, `.response_cookie`, `.termination`, `.connections` and `.queues`. HTTP status `0` means no response and remains an attribute; it does not become an HTTP status. Request `-`/`<BADREQ>` yields no HTTP request fields. Timers allow `-1`; connection/queue counters are nonnegative, including the standard `+` marker. Header captures remain in raw content.

The standard request date has a year but no UTC offset. It is validated and retained as `haproxy.time`, with an ambiguity diagnostic and no normalized instant. The Syslog emission time is never substituted for the request time. A selected extended date variant with a space and explicit numeric offset is also supported, for example `[15/Oct/2026:08:00:00.123 +0000]`. `logasap` byte counts beginning with `+` become `haproxy.bytes_lower_bound`, with a diagnostic and no exact response-byte value. The reader does not estimate an exact count.

Custom `log-format`, TCP/HTTPS extensions, CLF variants and JSON exports require other readers or future work.

Primary reference: [HAProxy 3.0 configuration manual, HTTP log format](https://www.haproxy.org/download/3.0/doc/configuration.txt).

## WAF: ModSecurity 2.x native audit

Explicit ModSecurity audit mode groups serial native `--token-A--` through matching `--token-Z--` sections into one event. Tokens contain ASCII letters/digits. Framing is disabled in every other mode. The reader extracts:

- A: explicit-offset timestamp, transaction ID, validated source/destination addresses and ports; extra values use `waf.id`, `.dst`, `.spt`, `.dpt`, `.boundary`.
- B, if recorded: request line, User-Agent and Referer.
- F, if recorded: actual response status. Content-Length is not treated as an observed response-byte count.
- H, if recorded: ordered `waf.messages` array and `waf.action`. WAF findings stay evidence attributes and do not create parsing warnings.

Other sections, bodies, blank lines and line endings remain in the raw event. Optional sections may be absent without a warning. Invalid A/B/F fields yield independent diagnostics and preserve valid extractions. A missing matching Z terminator yields a Partial event. A new A boundary flushes the incomplete previous event before starting the next; mismatched section IDs and duplicate sections produce diagnostics and are excluded from extraction (first section wins). The reader follows native delimiters; it does not reconstruct corrupted/interleaved transactions or infer intent from alert text. Lines outside a transaction, including blank lines, remain individual Unparsed events.

The source remains read-only. Each event retains the exact source byte sequence, the first physical line and byte offset. The inspector also reports its final physical line. Raw display removes only the final line terminator; internal CRLF/LF remain intact. Source SHA-256 covers every byte exactly once. Progress counts physical lines; event counts count grouped events. A final unterminated line is retained. Invalid UTF-8 produces a display warning while retaining original bytes.

Both individual lines and aggregate multiline events are limited to **1 MiB including terminators**. Exceeding either limit rolls back the entire append, retaining the previous case, source summaries, attributes and contiguous event IDs. Cancellation is checked between physical lines. This bounds framing memory without truncating evidence. Large transactions must use an alternative tool or a later large-volume reader.

Native ModSecurity 3.x boundaries, JSON audit variants, concurrent-audit directory/index import and arbitrary stack-trace grouping are outside scope.

Primary reference: [ModSecurity 2 data formats](https://github.com/owasp-modsecurity/ModSecurity/wiki/ModSecurity-2-Data-Formats).

## Exploration and case compatibility

Reader attributes appear in the inspector and support copy/exact-scalar filtering. The advanced Attribute / JSON field control accepts names such as `firewall.DPT` and `syslog.severity`. It shares the existing parameterized `jsonFilter`/`jsonFields` IPC storage contract. Objects/arrays remain inspectable; exact filters index top-level scalar attributes only. Global search includes decoded scalar attributes and complete raw content. Attribute names are case-sensitive; exact scalar values use the existing string representation. List pages remain bounded previews without full attributes.

Source, reader, parsing, warning, attribute and UTC filters can be combined across all sources, including after save/reopen. Undated records are visible with Missing / invalid timestamp and excluded from UTC ranges. The compact table and on-demand Statistics remain unchanged in structure.

Case storage remains schema version 1; 0.3.0 opens MVP 1 cases without migration. New optional summary/detail metadata adds physical-line counts/ranges; old source summaries fall back to one event per line. An older application does not understand the new reader modes: use 0.3.0 or later to reopen cases containing these sources. No downgrade compatibility is promised.
