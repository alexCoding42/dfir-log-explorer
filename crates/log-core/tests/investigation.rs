use dfir_log_core::*;
use std::{io::Write, sync::atomic::AtomicBool};

fn query() -> Query {
    Query {
        limit: 2,
        ..Default::default()
    }
}
fn import(engine: &mut Engine, lines: &[u8]) {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(lines).unwrap();
    engine
        .import(
            file.path(),
            ParserMode::JsonLines,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
}
fn fixture() -> Engine {
    let mut engine = Engine::new().unwrap();
    import(&mut engine, b"{\"timestamp\":\"2026-10-19T10:00:00Z\",\"source_ip\":\"192.0.2.1\",\"path\":\"/login\",\"method\":\"POST\",\"status\":401}\n{\"timestamp\":\"2026-10-19T10:00:01Z\",\"source_ip\":\"192.0.2.1\",\"path\":\"/login\",\"method\":\"GET\",\"status\":200}\n{\"source_ip\":\"192.0.2.2\",\"path\":\"/other\"}\n");
    import(&mut engine, b"{\"timestamp\":\"2026-10-19T10:00:02Z\",\"source_ip\":\"192.0.2.1\",\"path\":\"/other\",\"method\":\"POST\"}\n{\"message\":\"missing fields\"}\n");
    engine
}
#[test]
fn scoped_breakdowns_count_all_pages_and_drill_down_without_losing_filters() {
    let engine = fixture();
    let page = engine.query(&query()).unwrap();
    assert_eq!(page.rows.len(), 2);
    assert_eq!(page.total, 5);
    let stats = page.statistics;
    assert_eq!(
        (
            stats.missing_ip,
            stats.missing_path,
            stats.missing_timestamp
        ),
        (1, 1, 2)
    );
    assert_eq!(
        (stats.top_ips[0].value.as_str(), stats.top_ips[0].count),
        ("192.0.2.1", 3)
    );
    assert_eq!(
        stats
            .top_paths
            .iter()
            .map(|g| (g.value.as_str(), g.count))
            .collect::<Vec<_>>(),
        vec![("/login", 2), ("/other", 2)]
    );
    assert_eq!(
        stats
            .source_counts
            .iter()
            .map(|g| g.count)
            .collect::<Vec<_>>(),
        vec![3, 2]
    );
    assert_eq!(stats.reader_counts[0].value, "json_lines");
    let base = Query {
        field_filter: Some(FieldFilter {
            field: SearchField::Method,
            value: "POST".into(),
        }),
        field_filters: vec![FieldFilter {
            field: SearchField::SourceIp,
            value: "192.0.2.1".into(),
        }],
        ..query()
    };
    let scoped = engine.query(&base).unwrap();
    assert_eq!(scoped.total, 2);
    for group in scoped.statistics.top_paths {
        let mut clicked = base.clone();
        clicked.field_filters.push(FieldFilter {
            field: SearchField::Path,
            value: group.filter_value.unwrap(),
        });
        assert_eq!(engine.query(&clicked).unwrap().total, group.count);
    }
    let scoped = engine
        .query(&Query {
            source_id: Some(2),
            ..query()
        })
        .unwrap();
    assert_eq!(scoped.statistics.missing_ip, 1);
    assert_eq!(scoped.statistics.top_ips[0].count, 1);
    let empty = engine
        .query(&Query {
            search: "unmatched".into(),
            ..query()
        })
        .unwrap();
    assert_eq!(empty.statistics.missing_path, 0);
    assert!(empty.statistics.top_paths.is_empty());
}
#[test]
fn timeline_ranges_are_half_open_with_undated_events_retained_outside_time_filters() {
    let engine = fixture();
    let from = engine.detail(1).unwrap().event.fields.timestamp_ms.unwrap();
    let selected = engine
        .query(&Query {
            time_from: Some(from),
            time_to: Some(from + 1000),
            source_id: Some(1),
            ..query()
        })
        .unwrap();
    assert_eq!(selected.total, 1);
    assert_eq!(selected.rows[0].id, 1);
    assert_eq!(
        selected
            .statistics
            .timeline
            .iter()
            .map(|b| b.parsed + b.partial + b.unparsed)
            .sum::<u64>(),
        1
    );
    assert_eq!(
        engine
            .query(&Query {
                has_timestamp: Some(false),
                ..query()
            })
            .unwrap()
            .total,
        2
    );
    assert!(engine
        .query(&Query {
            time_from: Some(from + 9000),
            time_to: Some(from + 10000),
            ..query()
        })
        .unwrap()
        .statistics
        .timeline
        .is_empty());
}
#[test]
fn aggregate_labels_are_bounded_and_long_values_do_not_become_truncated_filters() {
    let mut engine = Engine::new().unwrap();
    let records = format!(
        "{}\n{}\n",
        serde_json::json!({"path":"x".repeat(5000)}),
        serde_json::json!({"path":"' OR true --"})
    );
    import(&mut engine, records.as_bytes());
    let stats = engine.query(&query()).unwrap().statistics;
    let long = stats
        .top_paths
        .iter()
        .find(|g| g.filter_value.is_none())
        .unwrap();
    assert_eq!(long.value.chars().count(), 240);
    let literal = stats
        .top_paths
        .iter()
        .find(|g| g.filter_value.is_some())
        .unwrap();
    assert_eq!(
        engine
            .query(&Query {
                field_filters: vec![FieldFilter {
                    field: SearchField::Path,
                    value: literal.filter_value.clone().unwrap()
                }],
                ..query()
            })
            .unwrap()
            .total,
        1
    );
}
#[test]
fn searches_survive_case_promotion_reopen_update_and_delete_without_changing_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.dfir");
    let mut engine = fixture();
    let raw = engine.detail(1).unwrap().raw;
    let query = Query {
        search: "login".into(),
        search_field: SearchField::Path,
        source_id: Some(1),
        parser: Some(ParserMode::JsonLines),
        has_timestamp: Some(true),
        json_filter: Some(JsonFilter {
            key: "method".into(),
            value: "POST".into(),
        }),
        field_filter: Some(FieldFilter {
            field: SearchField::Method,
            value: "POST".into(),
        }),
        field_filters: vec![FieldFilter {
            field: SearchField::SourceIp,
            value: "192.0.2.1".into(),
        }],
        sort_by: SortField::Timestamp,
        descending: true,
        offset: 8,
        ..query()
    };
    let search = engine
        .put_saved_search(None, "  Login investigation  ", query)
        .unwrap();
    assert_eq!(search.name, "Login investigation");
    assert_eq!(search.query.offset, 0);
    engine.save_case(&path).unwrap();
    drop(engine);
    let engine = Engine::open_case(&path).unwrap();
    let restored = engine.saved_search_query(search.id).unwrap();
    assert_eq!(engine.query(&restored).unwrap().total, 1);
    assert!(restored.descending);
    let mut updated = restored.clone();
    updated.search = "other".into();
    engine
        .put_saved_search(Some(search.id), "Renamed", updated)
        .unwrap();
    drop(engine);
    let engine = Engine::open_case(&path).unwrap();
    assert_eq!(engine.saved_searches().unwrap()[0].name, "Renamed");
    assert_eq!(engine.detail(1).unwrap().raw, raw);
    assert_eq!(engine.case_summary().unwrap().sources.len(), 2);
    engine.delete_saved_search(search.id).unwrap();
    drop(engine);
    assert!(Engine::open_case(&path)
        .unwrap()
        .saved_searches()
        .unwrap()
        .is_empty());
}
#[test]
fn old_cases_remain_readable_and_gain_the_optional_extension_only_on_first_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.dfir");
    let mut engine = fixture();
    engine.save_case(&path).unwrap();
    drop(engine);
    let before = std::fs::read(&path).unwrap();
    let engine = Engine::open_case(&path).unwrap();
    assert!(engine.saved_searches().unwrap().is_empty());
    drop(engine);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let engine = Engine::open_case(&path).unwrap();
    engine.put_saved_search(None, "First", query()).unwrap();
    drop(engine);
    let db = duckdb::Connection::open(&path).unwrap();
    let version: i32 = db
        .query_row("SELECT schema_version FROM case_meta", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 1);
}
#[test]
fn invalid_names_queries_and_unavailable_sources_are_rejected() {
    let engine = fixture();
    for name in ["   ".to_string(), "x".repeat(121)] {
        assert!(engine.put_saved_search(None, &name, query()).is_err());
    }
    let saved = engine.put_saved_search(None, "Unique", query()).unwrap();
    assert!(engine.put_saved_search(None, "UNIQUE", query()).is_err());
    assert!(engine
        .put_saved_search(Some(900), "Other", query())
        .is_err());
    assert!(engine
        .put_saved_search(
            None,
            "Missing",
            Query {
                source_id: Some(999),
                ..query()
            }
        )
        .is_err());
    assert!(engine
        .put_saved_search(
            None,
            "Invalid",
            Query {
                time_from: Some(10),
                time_to: Some(1),
                ..query()
            }
        )
        .is_err());
    assert!(engine.saved_search_query(saved.id + 1).is_err());
    assert!(engine
        .query(&Query {
            field_filters: (0..17)
                .map(|_| FieldFilter {
                    field: SearchField::Path,
                    value: "x".into()
                })
                .collect(),
            ..query()
        })
        .is_err());
}
#[test]
fn malformed_future_and_missing_source_definitions_never_broaden_the_query() {
    for mode in ["version", "payload", "source", "unknown_filter"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.dfir");
        let mut engine = fixture();
        let saved = engine
            .put_saved_search(
                None,
                "Test",
                Query {
                    source_id: Some(1),
                    ..query()
                },
            )
            .unwrap();
        engine.save_case(&path).unwrap();
        drop(engine);
        {
            let db = duckdb::Connection::open(&path).unwrap();
            if mode == "source" {
                db.execute("DELETE FROM sources WHERE id = 1", []).unwrap();
            } else {
                let payload = if mode == "unknown_filter" {
                    let mut definition = serde_json::to_value(&saved).unwrap();
                    definition["query"]["unrecognizedFilter"] =
                        serde_json::json!("must not be ignored");
                    definition.to_string()
                } else if mode == "version" {
                    let mut future = saved.clone();
                    future.version = 99;
                    serde_json::to_string(&future).unwrap()
                } else {
                    "broken".into()
                };
                db.execute(
                    "UPDATE saved_searches SET payload = ?",
                    duckdb::params![payload],
                )
                .unwrap();
            }
        }
        let engine = Engine::open_case(&path).unwrap();
        if mode == "source" {
            assert!(matches!(
                engine.saved_search_query(saved.id),
                Err(EngineError::SavedSearchSourceMissing)
            ));
        } else {
            assert!(matches!(
                engine.saved_searches(),
                Err(EngineError::UnsupportedSavedSearch)
            ));
        }
        assert_eq!(engine.query(&query()).unwrap().total, 5);
    }
}

#[test]
fn oversized_serialized_queries_are_rejected_before_creating_storage() {
    let engine = fixture();
    let large = Query {
        field_filters: (0..16)
            .map(|_| FieldFilter {
                field: SearchField::Path,
                value: "界".repeat(4096),
            })
            .collect(),
        ..query()
    };
    assert!(matches!(
        engine.put_saved_search(None, "Too large", large),
        Err(EngineError::SavedSearchTooLarge)
    ));
    assert!(engine.saved_searches().unwrap().is_empty());
}

#[test]
fn reader_breakdown_includes_unrecognized_records_and_filters_them_exactly() {
    let mut engine = fixture();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"not an apache record\n").unwrap();
    engine
        .import(
            file.path(),
            ParserMode::Apache,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let all = engine.query(&query()).unwrap();
    assert!(all
        .statistics
        .reader_counts
        .iter()
        .any(|g| g.value == "unparsed" && g.count == 1));
    let selected = engine
        .query(&Query {
            field_filters: vec![FieldFilter {
                field: SearchField::Parser,
                value: "unparsed".into(),
            }],
            ..query()
        })
        .unwrap();
    assert_eq!(selected.total, 1);
    assert_eq!(selected.rows[0].fields.parser, "unparsed");
}
