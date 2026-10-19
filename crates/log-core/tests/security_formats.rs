use dfir_log_core::*;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

const SYSLOG: &str = include_str!("../../../tests/fixtures/syslog.log");
const FIREWALL: &str = include_str!("../../../tests/fixtures/iptables.log");
const HAPROXY: &str = include_str!("../../../tests/fixtures/haproxy.log");
const AUDIT: &str = include_str!("../../../tests/fixtures/modsecurity-audit.log");
fn file(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    file
}
fn import(engine: &mut Engine, text: &[u8], mode: ParserMode) -> SourceSummary {
    engine
        .import(file(text).path(), mode, &AtomicBool::new(false), |_| {})
        .unwrap()
}
fn query() -> Query {
    Query {
        limit: 500,
        ..Default::default()
    }
}
fn attr(fields: &ParsedFields, key: &str) -> serde_json::Value {
    fields.json_fields.as_ref().unwrap()[key].clone()
}

#[test]
fn syslog_offsets_nil_values_structured_data_and_bsd_ambiguity() {
    let lines: Vec<_> = SYSLOG.lines().collect();
    let parsed = parse_line(lines[0], ParserMode::Syslog);
    assert_eq!(parsed.parse_status, ParseStatus::Parsed);
    assert_eq!(
        parsed.timestamp.as_deref(),
        Some("2026-10-15T10:00:00+02:00")
    );
    assert_eq!(attr(&parsed, "syslog.facility"), 20);
    assert_eq!(attr(&parsed, "syslog.severity"), 5);
    assert_eq!(
        attr(&parsed, "syslog.sd.origin@32473.note"),
        "quoted\" and bracket]"
    );
    let nil = parse_line(lines[1], ParserMode::Syslog);
    assert_eq!(nil.parse_status, ParseStatus::Parsed);
    assert_eq!(nil.timestamp_ms, None);
    let bsd = parse_line(lines[2], ParserMode::Syslog);
    assert_eq!(bsd.parse_status, ParseStatus::Partial);
    assert_eq!(bsd.timestamp_ms, None);
    assert_eq!(attr(&bsd, "syslog.pid"), "42");
    let bad = parse_line(lines[3], ParserMode::Syslog);
    assert_eq!(bad.parse_status, ParseStatus::Partial);
    assert!(bad.parse_error.unwrap().contains("priority"));
    assert_eq!(
        parse_line(lines[4], ParserMode::Syslog).parse_status,
        ParseStatus::Unparsed
    );
}
#[test]
fn malformed_syslog_sd_dates_versions_and_duplicates_are_diagnosed() {
    for tail in [
        "[id key=\"unfinished]",
        "[id key=\"bad\\q\"]",
        "[id]trailing",
        "[id key=\"a\" key=\"b\"]",
        "[id][id]",
        "[id\u{0001} key=\"value\"]",
        "[id key\u{0001}=\"value\"]",
        "[a.b c=\"first\"][a b.c=\"second\"]",
    ] {
        let p = parse_line(
            &format!("<1>1 2026-10-15T08:00:00Z host app - - {tail}"),
            ParserMode::Syslog,
        );
        assert_eq!(p.parse_status, ParseStatus::Partial, "{tail}");
        assert!(p.timestamp_ms.is_some());
    }
    for line in [
        "<1>2 2026-10-15T08:00:00Z host app - - - text",
        "<1>1 2026-10-15T08:00:00-00:00 host app - - - text",
        "Feb 30 08:00:00 host app: text",
    ] {
        assert_eq!(
            parse_line(line, ParserMode::Syslog).parse_status,
            ParseStatus::Partial
        );
    }
    let p = parse_line(
        "<1>1 - host app - - [id a=\"x\"][other b=\"y\"]",
        ParserMode::Syslog,
    );
    assert_eq!(p.parse_status, ParseStatus::Parsed);
    assert_eq!(attr(&p, "syslog.sd.other.b"), "y");
    let literal_dash = parse_line("<1>1 - host app - - - -", ParserMode::Syslog);
    assert_eq!(attr(&literal_dash, "syslog.message"), "-");
}
#[test]
fn firewall_ipv4_ipv6_and_independent_field_validation() {
    let lines: Vec<_> = FIREWALL.lines().collect();
    let p = parse_line(lines[0], ParserMode::Iptables);
    assert_eq!(p.parse_status, ParseStatus::Parsed);
    assert_eq!(p.source_ip.as_deref(), Some("192.0.2.10"));
    assert_eq!(p.protocol.as_deref(), Some("TCP"));
    assert_eq!(attr(&p, "firewall.DPT"), "443");
    assert_eq!(p.status, None);
    assert_eq!(p.response_bytes, None);
    assert_eq!(
        parse_line(lines[1], ParserMode::Iptables)
            .source_ip
            .as_deref(),
        Some("2001:db8::10")
    );
    let p = parse_line(lines[2], ParserMode::Iptables);
    assert_eq!(p.parse_status, ParseStatus::Partial);
    assert_eq!(p.source_ip, None);
    assert_eq!(attr(&p, "firewall.DPT"), "443");
    assert!(p.parse_error.unwrap().contains("port"));
    assert_eq!(
        parse_line("SRC=192.0.2.1", ParserMode::Iptables).parse_status,
        ParseStatus::Unparsed
    );
}
#[test]
fn haproxy_standard_and_offset_dates_ipv6_and_logasap() {
    let lines: Vec<_> = HAPROXY.lines().collect();
    let p = parse_line(lines[0], ParserMode::Haproxy);
    assert_eq!(p.parse_status, ParseStatus::Parsed);
    assert_eq!(p.path.as_deref(), Some("/synthetic"));
    assert_eq!(p.response_bytes, Some(512));
    assert_eq!(attr(&p, "haproxy.backend"), "backend");
    let p = parse_line(lines[1], ParserMode::Haproxy);
    assert_eq!(p.parse_status, ParseStatus::Partial);
    assert_eq!(p.timestamp_ms, None);
    assert_eq!(p.source_ip.as_deref(), Some("2001:db8::10"));
    assert_eq!(p.status, Some(403));
    let p = parse_line(lines[2], ParserMode::Haproxy);
    assert_eq!(p.parse_status, ParseStatus::Partial);
    assert_eq!(p.response_bytes, None);
    assert_eq!(p.status, None);
    assert_eq!(attr(&p, "haproxy.bytes_lower_bound"), "128");
    let bad = lines[0]
        .replace("0/0/1/2/3", "oops")
        .replace("200 512", "999 -5");
    let p = parse_line(&bad, ParserMode::Haproxy);
    assert_eq!(p.parse_status, ParseStatus::Partial);
    assert_eq!(p.path.as_deref(), Some("/synthetic"));
    assert_eq!(p.status, None);
    assert_eq!(p.response_bytes, None);
}
#[test]
fn native_audit_groups_transactions_without_losing_source_positions_or_bytes() {
    let mut engine = Engine::new().unwrap();
    let mut progress = Vec::new();
    let summary = engine
        .import(
            file(AUDIT.as_bytes()).path(),
            ParserMode::Modsecurity,
            &AtomicBool::new(false),
            |p| progress.push(p),
        )
        .unwrap();
    assert_eq!(summary.total_events, 3);
    assert_eq!(summary.total_lines, AUDIT.lines().count() as u64);
    assert_eq!(summary.parsed_events, 2);
    assert_eq!(summary.partial_events, 1);
    assert_eq!(
        summary.sha256,
        format!("{:x}", Sha256::digest(AUDIT.as_bytes()))
    );
    assert_eq!(progress.last().unwrap().lines_read, summary.total_lines);
    let first = engine.detail(1).unwrap();
    assert_eq!(first.event.line_number, 1);
    assert_eq!(first.end_line_number, 19);
    assert_eq!(first.event.fields.status, Some(403));
    assert_eq!(
        first.event.fields.user_agent.as_deref(),
        Some("SyntheticAgent/1.0")
    );
    assert!(
        first.event.fields.parse_error.is_none(),
        "WAF findings are not parsing warnings"
    );
    assert_eq!(
        attr(&first.event.fields, "waf.messages")
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let second = engine.detail(2).unwrap();
    assert_eq!(second.event.line_number, 20);
    assert_eq!(second.end_line_number, 26);
    assert_eq!(
        second.event.byte_offset,
        AUDIT.find("--def456-A--").unwrap() as u64
    );
    assert_eq!(
        first.event.fields.timestamp_ms.unwrap() + 1000,
        second.event.fields.timestamp_ms.unwrap()
    );
    let last = engine.detail(3).unwrap();
    assert_eq!(last.event.fields.path.as_deref(), Some("/incomplete"));
    assert!(last
        .event
        .fields
        .parse_error
        .unwrap()
        .contains("missing matching Z"));
    assert_eq!(
        engine
            .query(&Query {
                offset: 1,
                ..query()
            })
            .unwrap()
            .rows[0]
            .id,
        2
    );
}
#[test]
fn audit_resynchronizes_and_never_extracts_from_mismatched_or_duplicate_sections() {
    let broken = "--a-A--\n[15/Oct/2026:08:00:00 +0000] id 192.0.2.1 1 192.0.2.2 2\n--wrong-B--\nGET /wrong HTTP/1.1\n--a-B--\nGET /first HTTP/1.1\n--a-B--\nGET /duplicate HTTP/1.1\n--b-A--\n[15/Oct/2026:08:00:00 +0000] id 192.0.2.1 1 192.0.2.2 2\n--b-Z--";
    let mut engine = Engine::new().unwrap();
    let s = import(&mut engine, broken.as_bytes(), ParserMode::Modsecurity);
    assert_eq!(s.total_events, 2);
    let a = engine.detail(1).unwrap();
    assert_eq!(a.event.fields.path.as_deref(), Some("/first"));
    assert_eq!(a.end_line_number, 8);
    assert_eq!(engine.detail(2).unwrap().event.line_number, 9);
    assert_eq!(engine.detail(2).unwrap().end_line_number, 11);
    assert_eq!(a.event.fields.parse_status, ParseStatus::Partial);
}
#[test]
fn multiline_crlf_invalid_encoding_orphans_and_final_line_are_retained() {
    let bytes = b"orphan\r\n--a-A--\r\n[15/Oct/2026:08:00:00 +0000] id 192.0.2.1 1 192.0.2.2 2\r\n--a-H--\r\nMessage: invalid \xff\r\n--a-Z--";
    let mut engine = Engine::new().unwrap();
    let s = import(&mut engine, bytes, ParserMode::Modsecurity);
    assert_eq!(s.total_events, 2);
    assert_eq!(s.total_lines, 6);
    assert_eq!(s.unparsed_events, 1);
    let d = engine.detail(2).unwrap();
    assert_eq!(d.event.line_number, 2);
    assert_eq!(d.end_line_number, 6);
    assert_eq!(d.event.byte_offset, 8);
    assert_eq!(
        d.raw_hex.unwrap(),
        bytes[8..]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert!(d.raw.contains("\r\n"));
    assert!(d
        .event
        .fields
        .parse_error
        .unwrap()
        .contains("Invalid UTF-8"));
    // Explicit single-line modes never acquire audit framing.
    let s = import(&mut engine, bytes, ParserMode::Text);
    assert_eq!(s.total_events, 6);
}
#[test]
fn multiline_limit_and_cancellation_roll_back_saved_case_and_contiguous_ids() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rollback.dfir");
    let mut engine = Engine::new().unwrap();
    import(&mut engine, FIREWALL.as_bytes(), ParserMode::Iptables);
    engine.save_case(&path).unwrap();
    let before = engine.query(&query()).unwrap().statistics;
    let long = format!(
        "--a-A--\n{}\n{}\n",
        "x".repeat(MAX_LINE_BYTES / 2),
        "y".repeat(MAX_LINE_BYTES / 2)
    );
    assert!(matches!(
        engine.import(
            file(long.as_bytes()).path(),
            ParserMode::Modsecurity,
            &AtomicBool::new(false),
            |_| {}
        ),
        Err(EngineError::RecordTooLong)
    ));
    let cancel = AtomicBool::new(false);
    assert!(matches!(
        engine.import(
            file(AUDIT.as_bytes()).path(),
            ParserMode::Modsecurity,
            &cancel,
            |_| cancel.store(true, Ordering::Relaxed)
        ),
        Err(EngineError::Cancelled)
    ));
    assert_eq!(
        engine.query(&query()).unwrap().statistics.total,
        before.total
    );
    assert_eq!(engine.case_summary().unwrap().sources.len(), 1);
    let s = import(&mut engine, AUDIT.as_bytes(), ParserMode::Modsecurity);
    assert_eq!(s.id, 2);
    assert_eq!(engine.detail(4).unwrap().event.line_number, 1);
    drop(engine);
    assert_eq!(
        Engine::open_case(&path)
            .unwrap()
            .query(&query())
            .unwrap()
            .total,
        6
    );
}
#[test]
fn security_sources_attributes_and_combined_filters_survive_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("security.dfir");
    let mut engine = Engine::new().unwrap();
    for (fixture, mode) in [
        (SYSLOG, ParserMode::Syslog),
        (FIREWALL, ParserMode::Iptables),
        (HAPROXY, ParserMode::Haproxy),
        (AUDIT, ParserMode::Modsecurity),
    ] {
        import(&mut engine, fixture.as_bytes(), mode);
    }
    engine.save_case(&path).unwrap();
    drop(engine);
    let engine = Engine::open_case(&path).unwrap();
    assert_eq!(engine.case_summary().unwrap().sources.len(), 4);
    let result = engine
        .query(&Query {
            source_id: Some(2),
            parser: Some(ParserMode::Iptables),
            has_timestamp: Some(true),
            json_filter: Some(JsonFilter {
                key: "firewall.DPT".into(),
                value: "443".into(),
            }),
            ..query()
        })
        .unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.rows[0].source_id, 2);
    assert!(
        result.rows[0].fields.json_fields.is_none(),
        "attributes must remain detail-only"
    );
    let decoded = engine
        .query(&Query {
            search: "quoted\" and bracket]".into(),
            ..query()
        })
        .unwrap();
    assert_eq!(decoded.total, 1);
    assert_eq!(
        engine
            .query(&Query {
                parser: Some(ParserMode::Modsecurity),
                ..query()
            })
            .unwrap()
            .total,
        3
    );
    assert_eq!(
        engine
            .query(&Query {
                json_filter: Some(JsonFilter {
                    key: "firewall.DPT' OR true --".into(),
                    value: "443".into()
                }),
                ..query()
            })
            .unwrap()
            .total,
        0
    );
}

#[test]
fn prior_source_summaries_without_physical_line_metadata_remain_readable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("prior-case.dfir");
    let mut engine = Engine::new().unwrap();
    import(&mut engine, b"prior line\n", ParserMode::Text);
    engine.save_case(&path).unwrap();
    drop(engine);
    {
        let connection = duckdb::Connection::open(&path).unwrap();
        let summary: String = connection
            .query_row("SELECT summary FROM sources WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&summary).unwrap();
        value.as_object_mut().unwrap().remove("totalLines");
        connection
            .execute(
                "UPDATE sources SET summary = ? WHERE id = 1",
                duckdb::params![value.to_string()],
            )
            .unwrap();
    }
    let reopened = Engine::open_case(&path).unwrap();
    assert_eq!(reopened.case_summary().unwrap().sources[0].total_lines, 0);
    assert_eq!(reopened.detail(1).unwrap().end_line_number, 1);
    assert_eq!(reopened.detail(1).unwrap().raw, "prior line");
}
