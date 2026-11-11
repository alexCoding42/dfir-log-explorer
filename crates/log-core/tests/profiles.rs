use dfir_log_core::*;
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

const SAMPLE: &str = include_str!("../../../tests/fixtures/application.log");
fn definition() -> ProfileDefinition {
    ProfileDefinition {
        version: 1,
        name: "Application log".into(),
        pattern: r"(?P<timestamp>[^|]+?) \| (?P<level>[A-Z]+) \| (?P<message>.*)".into(),
    }
}
fn file(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    file
}
fn selection(profile: &ProfileRevision) -> ProfileSelection {
    ProfileSelection {
        id: profile.id,
        revision: profile.revision,
    }
}
fn query() -> Query {
    Query {
        limit: 500,
        ..Default::default()
    }
}

#[test]
fn preview_and_import_share_fields_diagnostics_ranges_and_exact_definition() {
    let preview = preview_profile(&definition(), SAMPLE).unwrap();
    assert_eq!(preview.len(), 4);
    assert_eq!(preview[0].fields.parse_status, ParseStatus::Parsed);
    assert_eq!(preview[1].fields.parse_status, ParseStatus::Partial);
    assert_eq!(preview[2].fields.parse_status, ParseStatus::Unparsed);
    assert_eq!(preview[3].fields.timestamp_ms, Some(1_794_398_402_000));
    let mut engine = Engine::new().unwrap();
    let profile = engine.put_parsing_profile(None, definition()).unwrap();
    let source = file(SAMPLE.as_bytes());
    let imported = engine
        .import_with_profile(
            source.path(),
            selection(&profile),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(imported.profile.as_ref(), Some(&profile));
    assert_eq!(
        (
            imported.parsed_events,
            imported.partial_events,
            imported.unparsed_events
        ),
        (2, 1, 1)
    );
    assert_eq!(imported.total_lines, 4);
    for row in preview {
        let detail = engine.detail(row.line_number).unwrap();
        assert_eq!(detail.raw, row.raw.trim_end_matches(['\r', '\n']));
        assert_eq!(detail.event.byte_offset, row.byte_offset);
        assert_eq!(detail.event.fields.parse_status, row.fields.parse_status);
        assert_eq!(detail.event.fields.parse_error, row.fields.parse_error);
        assert_eq!(detail.event.fields.timestamp_ms, row.fields.timestamp_ms);
        assert_eq!(detail.event.fields.json_fields, row.fields.json_fields);
        assert_eq!(detail.end_line_number, row.line_number);
    }
    let filtered = engine
        .query(&Query {
            parser: Some(ParserMode::Custom),
            json_filter: Some(JsonFilter {
                key: "level".into(),
                value: "ERROR".into(),
            }),
            ..query()
        })
        .unwrap();
    assert_eq!(filtered.total, 1);
    assert_eq!(std::fs::read(source.path()).unwrap(), SAMPLE.as_bytes());
    assert!(engine
        .import(
            source.path(),
            ParserMode::Custom,
            &AtomicBool::new(false),
            |_| {}
        )
        .is_err());
}

#[test]
fn immutable_history_survives_promotion_and_reopen_without_reinterpreting_evidence() {
    let mut engine = Engine::new().unwrap();
    let first = engine.put_parsing_profile(None, definition()).unwrap();
    let source = file(SAMPLE.as_bytes());
    engine
        .import_with_profile(
            source.path(),
            selection(&first),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let original = engine.detail(1).unwrap().raw;
    let mut changed = definition();
    changed.pattern = r"(?P<message>.*)".into();
    let second = engine
        .put_parsing_profile(Some(selection(&first)), changed)
        .unwrap();
    assert_eq!(second.revision, 2);
    assert_ne!(first.sha256, second.sha256);
    assert!(matches!(
        engine.put_parsing_profile(Some(selection(&first)), definition()),
        Err(EngineError::ProfileConflict)
    ));
    // Historical revision remains explicitly selectable, even after editing.
    engine
        .import_with_profile(
            source.path(),
            selection(&first),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    engine
        .import_with_profile(
            source.path(),
            selection(&second),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("profiles.dfir");
    engine.save_case(&path).unwrap();
    drop(engine);
    let reopened = Engine::open_case(&path).unwrap();
    assert_eq!(
        reopened.parsing_profiles().unwrap(),
        vec![first.clone(), second.clone()]
    );
    let sources = reopened.case_summary().unwrap().sources;
    assert_eq!(sources[0].profile, Some(first.clone()));
    assert_eq!(sources[1].profile, Some(first));
    assert_eq!(sources[2].profile, Some(second));
    assert_eq!(reopened.detail(1).unwrap().raw, original);
    assert_eq!(
        reopened.detail(9).unwrap().event.fields.parse_status,
        ParseStatus::Parsed
    );
}

#[test]
fn validation_rejects_invalid_rules_versions_names_and_expensive_patterns() {
    for pattern in [
        "",
        "(",
        "(.*)",
        "(?P<x>a)(b)",
        "(?P<x>(a+)+)",
        "(?P<x>(?=a))",
        r"(?P<x>a)\1",
        "(?P<x>a{10000000})",
    ] {
        let mut d = definition();
        d.pattern = pattern.into();
        assert!(preview_profile(&d, "a").is_err(), "{pattern}");
    }
    for name in ["", "   ", "has\nnewline"] {
        let mut d = definition();
        d.name = name.into();
        assert!(preview_profile(&d, "").is_err());
    }
    let mut d = definition();
    d.version = 2;
    assert!(matches!(
        preview_profile(&d, ""),
        Err(EngineError::UnsupportedProfile)
    ));
    d = definition();
    d.pattern = "a".repeat(4097);
    assert!(preview_profile(&d, "").is_err());
    d.pattern = (0..33).map(|i| format!("(?P<f{i}>a)")).collect();
    assert!(preview_profile(&d, "").is_err());
    assert!(serde_json::from_str::<ProfileDefinition>(
        r#"{"version":1,"name":"Bad","pattern":"(?P<x>.*)","script":"run"}"#
    )
    .is_err());
}

#[test]
fn preview_enforces_bytes_and_physical_lines_without_silent_truncation() {
    assert!(preview_profile(&definition(), "").unwrap().is_empty());
    let mut d = definition();
    d.pattern = r"(?P<message>.*)".into();
    assert_eq!(preview_profile(&d, &"\n".repeat(100)).unwrap().len(), 100);
    assert!(matches!(
        preview_profile(&d, &"\n".repeat(101)),
        Err(EngineError::ProfilePreviewLimit)
    ));
    assert!(preview_profile(&d, &"é".repeat(32_768)).is_ok());
    assert!(matches!(
        preview_profile(&d, &"é".repeat(32_769)),
        Err(EngineError::ProfilePreviewLimit)
    ));
    let rows = preview_profile(&d, "é\r\n\nlast").unwrap();
    assert_eq!((rows[1].byte_offset, rows[2].byte_offset), (4, 5));
    assert_eq!(rows[0].raw, "é\r\n");
}

#[test]
fn normalized_fields_keep_invalid_captures_with_partial_diagnostics() {
    let d = ProfileDefinition { version: 1, name: "HTTP variation".into(), pattern: r"(?P<timestamp>\S+) (?P<source_ip>\S+) (?P<method>\S+) (?P<path>\S+) (?P<status>\S+) (?P<response_bytes>\S+)".into() };
    let rows = preview_profile(
        &d,
        "2026-11-11T12:00:00Z 192.0.2.1 GET / 200 12\nbad-date 192.0.2.2 POST /login 999 -1",
    )
    .unwrap();
    assert_eq!(rows[0].fields.status, Some(200));
    assert_eq!(rows[0].fields.response_bytes, Some(12));
    assert_eq!(rows[1].fields.parse_status, ParseStatus::Partial);
    assert_eq!(rows[1].fields.source_ip.as_deref(), Some("192.0.2.2"));
    assert_eq!(rows[1].fields.status, None);
    assert_eq!(
        rows[1].fields.json_fields.as_ref().unwrap()["status"],
        "999"
    );
    assert!(rows[1]
        .fields
        .parse_error
        .as_ref()
        .unwrap()
        .contains("response_bytes"));
}

#[test]
fn failed_and_cancelled_profile_imports_rollback_sources_attributes_and_evidence() {
    let mut engine = Engine::new().unwrap();
    let profile = engine.put_parsing_profile(None, definition()).unwrap();
    let source = file(SAMPLE.as_bytes());
    engine
        .import_with_profile(
            source.path(),
            selection(&profile),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    let summary = serde_json::to_string(&engine.case_summary().unwrap()).unwrap();
    let cancel = AtomicBool::new(false);
    let result = engine.import_with_profile(source.path(), selection(&profile), &cancel, |_| {
        cancel.store(true, Ordering::Relaxed);
    });
    assert!(matches!(result, Err(EngineError::Cancelled)));
    let mut bytes = SAMPLE.as_bytes().to_vec();
    bytes.extend(vec![b'a'; MAX_LINE_BYTES + 1]);
    let oversized = file(&bytes);
    assert!(matches!(
        engine.import_with_profile(
            oversized.path(),
            selection(&profile),
            &AtomicBool::new(false),
            |_| {}
        ),
        Err(EngineError::LineTooLong)
    ));
    assert!(engine
        .import_with_profile(
            source.path(),
            ProfileSelection {
                id: 999,
                revision: 1
            },
            &AtomicBool::new(false),
            |_| {}
        )
        .is_err());
    assert_eq!(
        serde_json::to_string(&engine.case_summary().unwrap()).unwrap(),
        summary
    );
    assert_eq!(engine.query(&query()).unwrap().total, 4);
    assert_eq!(engine.parsing_profiles().unwrap(), vec![profile]);
    assert_eq!(
        engine.detail(1).unwrap().raw,
        SAMPLE.lines().next().unwrap()
    );
}

#[test]
fn opening_legacy_case_does_not_create_profile_storage_and_unknown_versions_fail_explicitly() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.dfir");
    let mut engine = Engine::new().unwrap();
    engine.save_case(&path).unwrap();
    drop(engine);
    let before = std::fs::read(&path).unwrap();
    let reopened = Engine::open_case(&path).unwrap();
    assert!(reopened.parsing_profiles().unwrap().is_empty());
    drop(reopened);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let connection = duckdb::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE parsing_profiles (profile_id BIGINT, revision INTEGER, payload VARCHAR)",
        )
        .unwrap();
    connection.execute("INSERT INTO parsing_profiles VALUES (1, 1, ?)", [r#"{"id":1,"revision":1,"definition":{"version":99,"name":"Future","pattern":"(?P<x>.*)"},"sha256":"unknown"}"#]).unwrap();
    connection.execute_batch("CHECKPOINT").unwrap();
    drop(connection);
    let unknown = std::fs::read(&path).unwrap();
    assert!(matches!(
        Engine::open_case(&path),
        Err(EngineError::UnsupportedProfile)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), unknown);
}

#[test]
fn revisions_are_bounded_and_duplicate_profile_names_do_not_overwrite_history() {
    let engine = Engine::new().unwrap();
    let mut current = engine.put_parsing_profile(None, definition()).unwrap();
    assert!(engine.put_parsing_profile(None, definition()).is_err());
    for revision in 2..=20 {
        current = engine
            .put_parsing_profile(Some(selection(&current)), definition())
            .unwrap();
        assert_eq!(current.revision, revision);
    }
    assert!(matches!(
        engine.put_parsing_profile(Some(selection(&current)), definition()),
        Err(EngineError::ProfileStorageLimit)
    ));
    assert_eq!(engine.parsing_profiles().unwrap().len(), 20);
}

#[test]
fn profile_import_retains_crlf_invalid_utf8_and_original_bytes_on_disk() {
    let mut engine = Engine::new().unwrap();
    let profile = engine
        .put_parsing_profile(
            None,
            ProfileDefinition {
                pattern: r"(?P<message>.*)".into(),
                ..definition()
            },
        )
        .unwrap();
    let bytes = b"first\r\ninvalid \xff\nlast";
    let source = file(bytes);
    engine
        .import_with_profile(
            source.path(),
            selection(&profile),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(engine.detail(2).unwrap().event.byte_offset, 7);
    assert!(engine
        .detail(2)
        .unwrap()
        .event
        .fields
        .parse_error
        .unwrap()
        .contains("Invalid UTF-8"));
    assert_eq!(
        engine.detail(2).unwrap().raw_hex.as_deref(),
        Some("696e76616c696420ff0a")
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("raw.dfir");
    engine.save_case(&path).unwrap();
    drop(engine);
    let connection = duckdb::Connection::open(path).unwrap();
    let mut statement = connection
        .prepare("SELECT raw FROM events ORDER BY id")
        .unwrap();
    let retained = statement
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .concat();
    assert_eq!(retained, bytes);
    assert_eq!(std::fs::read(source.path()).unwrap(), bytes);
}

#[test]
fn malformed_profile_metadata_is_rejected_without_rewriting_case_files() {
    for corruption in ["digest", "row-id", "gap", "unknown-field", "oversized"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("corrupt.dfir");
        let mut engine = Engine::new().unwrap();
        engine.put_parsing_profile(None, definition()).unwrap();
        engine.save_case(&path).unwrap();
        drop(engine);
        let connection = duckdb::Connection::open(&path).unwrap();
        let payload: String = connection
            .query_row("SELECT payload FROM parsing_profiles", [], |r| r.get(0))
            .unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        match corruption {
            "digest" => value["sha256"] = "tampered".into(),
            "row-id" => value["id"] = 2.into(),
            "gap" => {
                value["revision"] = 2.into();
                connection
                    .execute_batch("UPDATE parsing_profiles SET revision = 2")
                    .unwrap();
            }
            "unknown-field" => value["script"] = "run".into(),
            "oversized" => value["definition"]["pattern"] = "a".repeat(16_385).into(),
            _ => unreachable!(),
        }
        connection
            .execute(
                "UPDATE parsing_profiles SET payload = ?",
                [value.to_string()],
            )
            .unwrap();
        connection.execute_batch("CHECKPOINT").unwrap();
        drop(connection);
        let before = std::fs::read(&path).unwrap();
        assert!(
            matches!(
                Engine::open_case(&path),
                Err(EngineError::UnsupportedProfile)
            ),
            "{corruption}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn total_profile_and_revision_limits_preserve_existing_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("limits.dfir");
    let mut engine = Engine::new().unwrap();
    let first = engine
        .put_parsing_profile(
            None,
            ProfileDefinition {
                pattern: "(?P<message>.*)".into(),
                ..definition()
            },
        )
        .unwrap();
    engine.save_case(&path).unwrap();
    drop(engine);
    // Populate valid immutable revisions directly to exercise the case-wide boundary
    // without repeatedly compiling every intermediate fixture during setup.
    let connection = duckdb::Connection::open(&path).unwrap();
    connection
        .execute_batch("DELETE FROM parsing_profiles")
        .unwrap();
    for id in 1..=50 {
        let mut revision = first.clone();
        revision.id = id;
        revision.definition.name = format!("Profile {id}");
        use sha2::{Digest, Sha256};
        revision.sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&revision.definition).unwrap())
        );
        connection
            .execute(
                "INSERT INTO parsing_profiles VALUES (?, 1, ?)",
                duckdb::params![id as i64, serde_json::to_string(&revision).unwrap()],
            )
            .unwrap();
    }
    drop(connection);
    let engine = Engine::open_case(&path).unwrap();
    assert!(matches!(
        engine.put_parsing_profile(None, definition()),
        Err(EngineError::ProfileStorageLimit)
    ));
    let stored = engine.parsing_profiles().unwrap();
    drop(engine);
    let connection = duckdb::Connection::open(&path).unwrap();
    for mut profile in stored {
        for revision in 2..=4 {
            profile.revision = revision;
            connection
                .execute(
                    "INSERT INTO parsing_profiles VALUES (?, ?, ?)",
                    duckdb::params![
                        profile.id as i64,
                        revision,
                        serde_json::to_string(&profile).unwrap()
                    ],
                )
                .unwrap();
        }
    }
    drop(connection);
    let engine = Engine::open_case(&path).unwrap();
    assert_eq!(engine.parsing_profiles().unwrap().len(), 200);
    assert!(matches!(
        engine.put_parsing_profile(Some(ProfileSelection { id: 1, revision: 4 }), definition()),
        Err(EngineError::ProfileStorageLimit)
    ));
    assert_eq!(engine.parsing_profiles().unwrap().len(), 200);
}
