use dfir_log_core::*;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

fn source(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    file
}

fn query(search: &str) -> Query {
    Query {
        search: search.into(),
        limit: PAGE_SIZE,
        ..Default::default()
    }
}

#[test]
fn mixed_import_is_lossless_searchable_and_paginated() {
    let bytes = include_bytes!("../../../tests/fixtures/apache-mixed.log");
    let file = source(bytes);
    let mut engine = Engine::new().unwrap();
    let summary = engine
        .import(
            file.path(),
            ParserMode::Auto,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(summary.total_events, 13);
    assert_eq!(summary.parsed_events, 11);
    assert_eq!(summary.text_events, 1);
    assert_eq!(summary.warning_events, 1);
    assert_eq!(summary.sha256, format!("{:x}", Sha256::digest(bytes)));
    let results = engine.query(&query("POST /LOGIN")).unwrap();
    assert_eq!(results.total, 2);
    assert_eq!(results.rows[0].line_number, 3);
    let second = engine
        .query(&Query {
            limit: 2,
            offset: 2,
            ..query("")
        })
        .unwrap();
    assert_eq!(second.total, 13);
    assert_eq!(second.rows[0].id, 3);
    let detail = engine.detail(12).unwrap();
    assert!(detail.raw.contains("[invalid date]"));
    assert!(detail.event.fields.parse_error.is_some());
    assert_eq!(
        engine
            .query(&Query {
                only_warnings: true,
                ..query("")
            })
            .unwrap()
            .total,
        1
    );
    assert!(engine.query(&query("' OR 1=1 --")).unwrap().rows.is_empty());
    assert_eq!(std::fs::read(file.path()).unwrap(), bytes);
}

#[test]
fn cancellation_rolls_back_and_keeps_previous_session() {
    let old = source(b"previous event\n");
    let next = source(b"new event\nnew event two\n");
    let mut engine = Engine::new().unwrap();
    engine
        .import(
            old.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let flag = AtomicBool::new(false);
    let result = engine.import(next.path(), ParserMode::Text, &flag, |_| {
        flag.store(true, Ordering::Relaxed)
    });
    assert!(matches!(result, Err(EngineError::Cancelled)));
    assert_eq!(engine.query(&query("previous")).unwrap().total, 1);
    assert_eq!(engine.query(&query("new event")).unwrap().total, 0);
}

#[test]
fn cancellation_after_streaming_progress_discards_already_appended_rows() {
    let old = source(b"previous event\n");
    let next =
        source(&b"new event with enough bytes to exercise progress updates\n".repeat(200_000));
    let mut engine = Engine::new().unwrap();
    engine
        .import(
            old.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let flag = AtomicBool::new(false);
    let mut progressed = false;
    let result = engine.import(next.path(), ParserMode::Text, &flag, |progress| {
        if progress.lines_read > 0 && progress.bytes_read < progress.total_bytes {
            progressed = true;
            flag.store(true, Ordering::Relaxed);
        }
    });
    assert!(
        progressed,
        "the test must cancel after actual streaming progress"
    );
    assert!(matches!(result, Err(EngineError::Cancelled)));
    assert_eq!(engine.query(&query("previous")).unwrap().total, 1);
    assert_eq!(engine.query(&query("new event")).unwrap().total, 0);
}

#[test]
fn queries_search_beyond_previews_and_source_changes_roll_back() {
    let mut engine = Engine::new().unwrap();
    let text = format!("{}needle\n", "a".repeat(300));
    let file = source(text.as_bytes());
    engine
        .import(
            file.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let page = engine.query(&query("needle")).unwrap();
    assert_eq!(page.total, 1);
    assert!(!page.rows[0].preview.contains("needle"));
    assert!(engine.detail(1).unwrap().raw.ends_with("needle"));
    let changing = source(b"original source\n");
    let result = engine.import(
        changing.path(),
        ParserMode::Text,
        &AtomicBool::new(false),
        |progress| {
            if progress.bytes_read == 0 {
                let mut writer = std::fs::OpenOptions::new()
                    .append(true)
                    .open(changing.path())
                    .unwrap();
                writer.write_all(b"added while importing\n").unwrap();
            }
        },
    );
    assert!(matches!(result, Err(EngineError::SourceChanged)));
    assert_eq!(engine.query(&query("needle")).unwrap().total, 1);
}

#[test]
fn preserves_invalid_utf8_blank_lines_crlf_and_unterminated_last_line() {
    let file = source(b"first\r\n\ninvalid \xff\nlast");
    let mut engine = Engine::new().unwrap();
    let summary = engine
        .import(
            file.path(),
            ParserMode::Auto,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(summary.total_events, 4);
    assert_eq!(summary.warning_events, 1);
    assert_eq!(engine.detail(1).unwrap().raw, "first");
    assert_eq!(engine.detail(2).unwrap().raw, "");
    let detail = engine.detail(3).unwrap();
    assert_eq!(detail.event.byte_offset, 8);
    assert_eq!(detail.raw_hex.as_deref(), Some("696e76616c696420ff0a"));
    assert_eq!(engine.detail(4).unwrap().raw, "last");
}

#[test]
fn oversized_lines_fail_without_partial_results_or_unbounded_allocation() {
    let bytes = vec![b'a'; MAX_LINE_BYTES + 1];
    let file = source(&bytes);
    let mut engine = Engine::new().unwrap();
    assert!(matches!(
        engine.import(
            file.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {}
        ),
        Err(EngineError::LineTooLong)
    ));
    assert_eq!(engine.query(&query("")).unwrap().total, 0);
}

#[test]
fn empty_file_and_literal_search_work() {
    let mut engine = Engine::new().unwrap();
    let file = source(b"");
    assert_eq!(
        engine
            .import(
                file.path(),
                ParserMode::Auto,
                &AtomicBool::new(false),
                |_| {}
            )
            .unwrap()
            .total_events,
        0
    );
    let file = source(b"100% [literal] _underscore_\nother\n");
    engine
        .import(
            file.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(engine.query(&query("% [literal] _")).unwrap().total, 1);
    assert!(matches!(
        engine.query(&Query {
            limit: 501,
            ..query("")
        }),
        Err(EngineError::InvalidQuery)
    ));
    assert!(matches!(
        engine.detail(999),
        Err(EngineError::EventNotFound)
    ));
}

#[test]
fn long_extracted_fields_cannot_inflate_pages_and_detail_remains_complete() {
    let path = format!("/{}", "a".repeat(32768));
    let agent = "b".repeat(32768);
    let line = format!("192.0.2.10 - - [13/Oct/2026:09:14:00 +0000] \"GET {path} HTTP/1.1\" 200 42 \"-\" \"{agent}\"\n");
    let file = source(line.repeat(20).as_bytes());
    let mut engine = Engine::new().unwrap();
    engine
        .import(
            file.path(),
            ParserMode::Auto,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let page = engine.query(&query("")).unwrap();
    assert_eq!(page.total, 20);
    assert!(serde_json::to_vec(&page).unwrap().len() < 1_000_000);
    let detail = engine.detail(1).unwrap();
    assert_eq!(detail.event.fields.path.as_deref(), Some(path.as_str()));
    assert_eq!(
        detail.event.fields.user_agent.as_deref(),
        Some(agent.as_str())
    );
    assert!(detail.raw.contains(&path));
    let beyond_end = engine
        .query(&Query {
            offset: 21,
            ..query("")
        })
        .unwrap();
    assert_eq!(beyond_end.total, 20);
    assert!(beyond_end.rows.is_empty());
}

#[test]
fn rejects_unsupported_archives_binary_and_utf16_sources_without_replacing_data() {
    let mut engine = Engine::new().unwrap();
    let old = source(b"keep this event\n");
    engine
        .import(
            old.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    for signature in [b"PK\x03\x04".as_slice(), b"ElfFile\0", b"\xff\xfe"] {
        let file = source(signature);
        assert!(matches!(
            engine.import(
                file.path(),
                ParserMode::Auto,
                &AtomicBool::new(false),
                |_| {}
            ),
            Err(EngineError::UnsupportedFormat)
        ));
        assert_eq!(engine.query(&query("keep this")).unwrap().total, 1);
    }
}

#[test]
fn parsing_statuses_fields_filters_and_aggregates_use_all_source_rows() {
    let file = source(include_bytes!("../../../tests/fixtures/apache-mixed.log"));
    let mut engine = Engine::new().unwrap();
    let summary = engine
        .import(
            file.path(),
            ParserMode::Auto,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(
        (
            summary.parsed_events,
            summary.partial_events,
            summary.unparsed_events
        ),
        (11, 1, 1)
    );
    let all = engine
        .query(&Query {
            limit: 2,
            ..query("")
        })
        .unwrap();
    assert_eq!(all.rows.len(), 2);
    let stats = &all.statistics;
    assert_eq!(
        (stats.total, stats.parsed, stats.partial, stats.unparsed),
        (13, 11, 1, 1)
    );
    assert_eq!(stats.warnings, 1);
    assert_eq!(stats.total_requests, 12);
    assert_eq!(stats.unique_ips, 8);
    assert_eq!(stats.responses, [7, 0, 4, 1]);
    assert_eq!(stats.missing_timestamp, 2);
    assert!(stats.timeline.len() <= 32);
    assert_eq!(
        stats
            .timeline
            .iter()
            .map(|b| b.parsed + b.partial + b.unparsed)
            .sum::<u64>(),
        11
    );
    assert_eq!(
        stats
            .timeline
            .iter()
            .map(|b| b.responses.iter().sum::<u64>())
            .sum::<u64>(),
        11
    );
    let partial = engine
        .query(&Query {
            parse_status: Some(ParseStatus::Partial),
            only_warnings: true,
            ..query("")
        })
        .unwrap();
    assert_eq!(partial.total, 1);
    assert_eq!(
        partial.rows[0].fields.source_ip.as_deref(),
        Some("192.0.2.14")
    );
    assert_eq!(partial.rows[0].fields.timestamp_ms, None);
    assert_eq!(partial.rows[0].fields.status, Some(200));
    let raw = engine
        .query(&Query {
            parse_status: Some(ParseStatus::Unparsed),
            search_field: SearchField::Raw,
            ..query("<img src=")
        })
        .unwrap();
    assert_eq!(raw.total, 1);
    assert_eq!(raw.rows[0].line_number, 11);
    assert_eq!(raw.statistics.timeline.len(), 0);
    let ip = engine
        .query(&Query {
            field_filter: Some(FieldFilter {
                field: SearchField::SourceIp,
                value: "198.51.100.24".into(),
            }),
            ..query("")
        })
        .unwrap();
    assert_eq!(ip.total, 3);
    assert_eq!(ip.statistics.responses, [0, 0, 3, 0]);
    let timestamp = engine
        .query(&Query {
            search_field: SearchField::Timestamp,
            ..query("2026-10-13T09:14:00")
        })
        .unwrap();
    assert_eq!(timestamp.total, 1); // ISO timestamp is extracted, absent from raw Apache timestamp.
    assert_eq!(
        engine
            .query(&Query {
                search_field: SearchField::Raw,
                ..query("2026-10-13T09:14:00")
            })
            .unwrap()
            .total,
        0
    );
    let first_ms = engine.detail(1).unwrap().event.fields.timestamp_ms.unwrap();
    let range = engine
        .query(&Query {
            time_from: Some(first_ms),
            time_to: Some(first_ms + 1000),
            ..query("")
        })
        .unwrap();
    assert_eq!(range.total, 1); // End is exclusive; invalid and missing timestamps are excluded.
    let sorted = engine
        .query(&Query {
            sort_by: SortField::Status,
            descending: true,
            limit: 2,
            ..query("")
        })
        .unwrap();
    assert_eq!(sorted.rows[0].fields.status, Some(502));
    assert_eq!(sorted.rows[1].fields.status, Some(404));
    assert_eq!(sorted.statistics.total, 13);
    assert!(engine
        .query(&Query {
            time_from: Some(2),
            time_to: Some(1),
            ..query("")
        })
        .is_err());
    assert!(engine
        .query(&Query {
            field_filter: Some(FieldFilter {
                field: SearchField::Path,
                value: "x".repeat(4097)
            }),
            ..query("")
        })
        .is_err());
    assert!(engine
        .query(&Query {
            time_from: Some(i64::MIN),
            ..query("")
        })
        .is_err());
    assert!(engine
        .query(&Query {
            field_filter: Some(FieldFilter {
                field: SearchField::Path,
                value: "' OR 1=1 --".into()
            }),
            ..query("")
        })
        .unwrap()
        .rows
        .is_empty());
}

#[test]
fn successful_import_invalidates_aggregate_cache_and_rollback_preserves_it() {
    let mut engine = Engine::new().unwrap();
    let old = source(b"old\n");
    engine
        .import(
            old.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(engine.query(&query("")).unwrap().statistics.parsed, 1);
    let next = source(b"next\nnext two\n");
    assert!(engine
        .import(
            next.path(),
            ParserMode::Text,
            &AtomicBool::new(true),
            |_| {}
        )
        .is_err());
    assert_eq!(engine.query(&query("")).unwrap().statistics.total, 1);
    engine
        .import(
            next.path(),
            ParserMode::Auto,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let stats = engine.query(&query("")).unwrap().statistics;
    assert_eq!((stats.total, stats.parsed, stats.unparsed), (3, 1, 2));
}

#[test]
fn old_query_payloads_remain_compatible_and_unknown_columns_are_rejected() {
    let legacy: Query =
        serde_json::from_str(r#"{"search":"","onlyWarnings":false,"offset":0,"limit":500}"#)
            .unwrap();
    assert!(legacy.is_original_order());
    assert!(serde_json::from_str::<Query>(r#"{"sortBy":"id; DROP TABLE events"}"#).is_err());
    assert!(serde_json::from_str::<Query>(r#"{"searchField":"nonexistent"}"#).is_err());
}

#[test]
fn sorting_and_statistics_are_global_beyond_the_first_page() {
    let common = "192.0.2.10 - - [13/Oct/2026:09:14:00 +0000] \"GET / HTTP/1.1\" 200 42\n";
    let data = format!(
        "{}{}",
        common.repeat(1000),
        common.replace("200 42", "503 42")
    );
    let file = source(data.as_bytes());
    let mut engine = Engine::new().unwrap();
    engine
        .import(
            file.path(),
            ParserMode::Apache,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let sorted = engine
        .query(&Query {
            limit: 1,
            sort_by: SortField::Status,
            descending: true,
            ..query("")
        })
        .unwrap();
    assert_eq!(sorted.rows[0].id, 1001);
    assert_eq!(sorted.statistics.total, 1001);
    assert_eq!(sorted.statistics.responses, [1000, 0, 0, 1]);
    let last = engine
        .query(&Query {
            offset: 1000,
            ..query("")
        })
        .unwrap();
    assert_eq!(last.rows.len(), 1);
    assert_eq!(last.rows[0].id, 1001);
    assert_eq!(
        last.statistics
            .timeline
            .iter()
            .map(|b| b.parsed)
            .sum::<u64>(),
        1001
    );
    assert_eq!(
        engine.query(&query("2026-10-13T09:14:00")).unwrap().total,
        1001
    );
}
