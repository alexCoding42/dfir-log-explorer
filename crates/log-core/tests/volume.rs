use dfir_log_core::*;
use serde_json::Value;
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

fn source(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(bytes).unwrap();
    f
}
fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    e.write_all(bytes).unwrap();
    e.finish().unwrap()
}
fn query() -> Query {
    Query {
        limit: PAGE_SIZE,
        ..Default::default()
    }
}
fn import(e: &mut Engine, bytes: &[u8]) -> SourceSummary {
    e.import(
        source(bytes).path(),
        ParserMode::Auto,
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap()
}
#[test]
fn gzip_members_preserve_decoded_evidence_and_separate_hashes() {
    let raw = b"one\r\ninvalid \xff\nlast";
    let mut compressed = gzip(&raw[..5]);
    compressed.extend(gzip(&raw[5..]));
    let mut e = Engine::new().unwrap();
    let summary = import(&mut e, &compressed);
    let mut reference = Engine::new().unwrap();
    let expected = import(&mut reference, raw);
    assert_eq!(summary.sha256, expected.sha256);
    assert_eq!(summary.total_bytes, raw.len() as u64);
    assert_eq!(summary.total_events, expected.total_events);
    let info = summary.compression.unwrap();
    assert_eq!(info.compressed_bytes, compressed.len() as u64);
    use sha2::{Digest, Sha256};
    assert_eq!(
        info.compressed_sha256,
        format!("{:x}", Sha256::digest(&compressed))
    );
    for id in 1..=summary.total_events {
        assert_eq!(
            serde_json::to_value(e.detail(id).unwrap()).unwrap(),
            serde_json::to_value(reference.detail(id).unwrap()).unwrap()
        );
    }
}
#[test]
fn corrupt_gzip_and_cancellation_never_publish() {
    let mut e = Engine::new().unwrap();
    import(&mut e, b"previous\n");
    let valid = gzip(b"next\n");
    for bytes in [valid[..valid.len() - 3].to_vec(), {
        let mut c = valid.clone();
        let n = c.len();
        c[n - 5] ^= 0xff;
        c
    }] {
        assert!(matches!(
            e.import(
                source(&bytes).path(),
                ParserMode::Text,
                &AtomicBool::new(false),
                |_| {}
            ),
            Err(EngineError::InvalidCompression)
        ));
        assert_eq!(e.query(&query()).unwrap().total, 1);
    }
    assert!(matches!(
        e.import(
            source(&valid).path(),
            ParserMode::Text,
            &AtomicBool::new(true),
            |_| {}
        ),
        Err(EngineError::Cancelled)
    ));
}
#[test]
fn resume_after_reopening_is_lossless_and_invisible_until_publication() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("case.dfir");
    let input = source(&b"one\r\ntwo \xff\n".repeat(10_005));
    let mut e = Engine::new().unwrap();
    import(&mut e, b"previous\n");
    e.save_case(&path).unwrap();
    let flag = AtomicBool::new(false);
    let result = e.import_resumable(input.path(), ParserMode::Text, None, &flag, |p| {
        if p.lines_read >= CHECKPOINT_EVENTS {
            flag.store(true, Ordering::Relaxed);
        }
    });
    assert!(
        matches!(result, Err(EngineError::ImportPaused)),
        "{result:?}"
    );
    assert_eq!(e.query(&query()).unwrap().total, 1);
    assert_eq!(e.case_summary().unwrap().sources.len(), 1);
    assert_eq!(
        e.pending_import().unwrap().unwrap().events,
        CHECKPOINT_EVENTS
    );
    assert!(matches!(
        e.import(
            input.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {}
        ),
        Err(EngineError::PendingImportExists)
    ));
    drop(e);
    let mut e = Engine::open_case(&path).unwrap();
    let resumed = e
        .import_resumable(
            input.path(),
            ParserMode::Apache,
            None,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(resumed.parser_mode, ParserMode::Text); // Stored reader wins.
    assert_eq!(resumed.total_events, 20_010);
    assert!(e.pending_import().unwrap().is_none());
    let mut reference = Engine::new().unwrap();
    import(&mut reference, b"previous\n");
    let uninterrupted = reference
        .import(
            input.path(),
            ParserMode::Text,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(resumed.sha256, uninterrupted.sha256);
    assert_eq!(
        e.query(&query()).unwrap().total,
        reference.query(&query()).unwrap().total
    );
    for id in [1, 2, 10_000, 10_001, 20_011] {
        assert_eq!(
            serde_json::to_value(e.detail(id).unwrap()).unwrap(),
            serde_json::to_value(reference.detail(id).unwrap()).unwrap()
        );
    }
}
#[test]
fn changed_source_is_rejected_and_discard_preserves_completed_sources() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("case.dfir");
    let mut e = Engine::new().unwrap();
    import(&mut e, b"previous\n");
    e.save_case(&path).unwrap();
    let input = source(&b"x\n".repeat(15_000));
    let flag = AtomicBool::new(false);
    let _ = e.import_resumable(input.path(), ParserMode::Text, None, &flag, |p| {
        if p.lines_read >= CHECKPOINT_EVENTS {
            flag.store(true, Ordering::Relaxed);
        }
    });
    std::fs::write(input.path(), b"y\n".repeat(15_000)).unwrap();
    assert!(matches!(
        e.import_resumable(
            input.path(),
            ParserMode::Text,
            None,
            &AtomicBool::new(false),
            |_| {}
        ),
        Err(EngineError::SourceChanged)
    ));
    assert_eq!(
        e.pending_import().unwrap().unwrap().events,
        CHECKPOINT_EVENTS
    );
    e.discard_pending_import().unwrap();
    assert!(e.pending_import().unwrap().is_none());
    assert_eq!(e.query(&query()).unwrap().total, 1);
}
#[test]
fn export_is_filtered_full_fidelity_all_pages_and_atomic() {
    let mut e = Engine::new().unwrap();
    let raw = format!(
        "{{\"path\":\"/{}\",\"user\":\"Zoë, \\\"Q\\\"\",\"keep\":true}}\r\n",
        "long".repeat(100)
    );
    let input = source(raw.repeat(600).as_bytes());
    e.import(
        input.path(),
        ParserMode::JsonLines,
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    import(&mut e, b"excluded\n");
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("out.jsonl");
    let q = Query {
        search: "Zoë".into(),
        offset: 500,
        limit: 1,
        ..query()
    };
    assert_eq!(
        e.export(
            &q,
            ExportFormat::JsonLines,
            true,
            &output,
            &AtomicBool::new(false),
            |_| {}
        )
        .unwrap(),
        600
    );
    let content = std::fs::read_to_string(&output).unwrap();
    let rows: Vec<Value> = content
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), 600);
    assert_eq!(rows[0]["event"]["raw"], raw);
    assert_eq!(rows[599]["event"]["id"], 600);
    assert!(rows[0]["event"].get("preview").is_none());
    assert!(rows[0]["source"]["sha256"].as_str().unwrap().len() == 64);
    let before = std::fs::read(&output).unwrap();
    assert!(e
        .export(
            &q,
            ExportFormat::Csv,
            false,
            &output,
            &AtomicBool::new(false),
            |_| {}
        )
        .is_err());
    assert_eq!(std::fs::read(&output).unwrap(), before);
    let cancelled = directory.path().join("cancelled.csv");
    let flag = AtomicBool::new(false);
    assert!(matches!(
        e.export(&q, ExportFormat::Csv, false, &cancelled, &flag, |p| {
            if p.events_written >= 500 {
                flag.store(true, Ordering::Relaxed);
            }
        }),
        Err(EngineError::ExportCancelled)
    ));
    assert!(!cancelled.exists());
    let csv = directory.path().join("out.csv");
    e.export(
        &q,
        ExportFormat::Csv,
        false,
        &csv,
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    let csv_text = std::fs::read_to_string(csv).unwrap();
    assert_eq!(csv_text.lines().count(), 601);
    assert!(!csv_text.contains("\"\"raw\"\""));
    assert_eq!(e.query(&query()).unwrap().total, 601);
}
#[test]
#[ignore = "subprocess worker used by abrupt_process_exit_recovers_checkpoint"]
fn ingestion_subprocess_worker() {
    let case = std::env::var_os("DFIR_TEST_CASE").unwrap();
    let input = std::env::var_os("DFIR_TEST_SOURCE").unwrap();
    let mut e = Engine::open_case(std::path::Path::new(&case)).unwrap();
    let _ = e.import_resumable(
        std::path::Path::new(&input),
        ParserMode::Text,
        None,
        &AtomicBool::new(false),
        |p| {
            if p.lines_read >= CHECKPOINT_EVENTS {
                std::process::exit(73);
            }
        },
    );
    panic!("worker must exit during ingestion");
}
#[test]
fn abrupt_process_exit_recovers_checkpoint() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("case.dfir");
    let input = source(&b"event\n".repeat(12_000));
    let mut e = Engine::new().unwrap();
    e.save_case(&path).unwrap();
    drop(e);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "ingestion_subprocess_worker"])
        .env("DFIR_TEST_CASE", &path)
        .env("DFIR_TEST_SOURCE", input.path())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
    let mut e = Engine::open_case(&path).unwrap();
    assert_eq!(e.query(&query()).unwrap().total, 0);
    assert_eq!(
        e.pending_import().unwrap().unwrap().events,
        CHECKPOINT_EVENTS
    );
    let summary = e
        .import_resumable(
            input.path(),
            ParserMode::Text,
            None,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(summary.total_events, 12_000);
    assert_eq!(e.query(&query()).unwrap().total, 12_000);
}

#[test]
fn resumed_gzip_retains_profile_revision_and_queryable_attributes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("case.dfir");
    let input = source(&gzip(&b"INFO|hello\r\n".repeat(10_005)));
    let mut e = Engine::new().unwrap();
    let a = e
        .put_parsing_profile(
            None,
            ProfileDefinition {
                version: 1,
                name: "App".into(),
                pattern: r"(?P<level>INFO)\|(?P<message>.*)".into(),
            },
        )
        .unwrap();
    e.save_case(&path).unwrap();
    let selection = ProfileSelection {
        id: a.id,
        revision: a.revision,
    };
    let flag = AtomicBool::new(false);
    assert!(matches!(
        e.import_resumable(
            input.path(),
            ParserMode::Custom,
            Some(selection),
            &flag,
            |p| {
                if p.lines_read >= CHECKPOINT_EVENTS {
                    flag.store(true, Ordering::Relaxed);
                }
            }
        ),
        Err(EngineError::ImportPaused)
    ));
    e.put_parsing_profile(
        Some(selection),
        ProfileDefinition {
            version: 1,
            name: "App".into(),
            pattern: r"(?P<other>.*)".into(),
        },
    )
    .unwrap();
    drop(e);
    let mut e = Engine::open_case(&path).unwrap();
    let summary = e
        .import_resumable(
            input.path(),
            ParserMode::Text,
            None,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(summary.profile.unwrap(), a);
    assert!(summary.compression.is_some());
    assert_eq!(
        e.query(&Query {
            json_filter: Some(JsonFilter {
                key: "level".into(),
                value: "INFO".into()
            }),
            ..query()
        })
        .unwrap()
        .total,
        10_005
    );
}

#[test]
fn invalid_checkpoint_version_is_explicit_and_can_be_discarded() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("case.dfir");
    let input = source(&b"event\n".repeat(10_005));
    let mut e = Engine::new().unwrap();
    e.save_case(&path).unwrap();
    let flag = AtomicBool::new(false);
    let _ = e.import_resumable(input.path(), ParserMode::Text, None, &flag, |p| {
        if p.lines_read >= CHECKPOINT_EVENTS {
            flag.store(true, Ordering::Relaxed);
        }
    });
    let mut checkpoint = e.pending_import().unwrap().unwrap();
    checkpoint.version = 99;
    drop(e);
    let connection = duckdb::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE ingestion_checkpoint SET state = ?",
            duckdb::params![serde_json::to_string(&checkpoint).unwrap()],
        )
        .unwrap();
    drop(connection);
    let e = Engine::open_case(&path).unwrap();
    assert!(matches!(
        e.pending_import(),
        Err(EngineError::InvalidCheckpoint)
    ));
    drop(e);
    // A syntactically valid checkpoint with a corrupt record boundary is rejected too.
    checkpoint.version = 1;
    checkpoint.bytes_read += 1;
    let connection = duckdb::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE ingestion_checkpoint SET state = ?",
            duckdb::params![serde_json::to_string(&checkpoint).unwrap()],
        )
        .unwrap();
    drop(connection);
    let e = Engine::open_case(&path).unwrap();
    assert!(matches!(
        e.pending_import(),
        Err(EngineError::InvalidCheckpoint)
    ));
    assert_eq!(e.query(&query()).unwrap().total, 0);
    e.discard_pending_import().unwrap();
    assert!(e.pending_import().unwrap().is_none());
}

#[test]
fn cached_pages_preserve_fields_and_invalidate_after_publication() {
    let mut e = Engine::new().unwrap();
    import(&mut e, b"first\n");
    let first = e.query(&query()).unwrap();
    let cached = e.query(&query()).unwrap();
    assert_eq!(
        serde_json::to_value(first.rows).unwrap(),
        serde_json::to_value(cached.rows).unwrap()
    );
    import(&mut e, b"second\n");
    assert_eq!(e.query(&query()).unwrap().total, 2);
    let directory = tempfile::tempdir().unwrap();
    e.save_case(&directory.path().join("case.dfir")).unwrap();
    let q = Query {
        search: "third".into(),
        ..query()
    };
    assert_eq!(e.query(&q).unwrap().total, 0);
    let input = source(b"third\n");
    e.import_resumable(
        input.path(),
        ParserMode::Text,
        None,
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    assert_eq!(e.query(&q).unwrap().total, 1);
    assert!(matches!(
        e.query(&Query {
            limit: 0,
            ..query()
        }),
        Err(EngineError::InvalidQuery)
    ));
}

#[test]
fn multiline_resume_uses_logical_record_boundary_after_reader_lookahead() {
    let directory = tempfile::tempdir().unwrap();
    let input = source(&b"--audit-A--\nunfinished audit transaction\r\n".repeat(10_005));
    let mut e = Engine::new().unwrap();
    e.save_case(&directory.path().join("case.dfir")).unwrap();
    let flag = AtomicBool::new(false);
    assert!(matches!(
        e.import_resumable(input.path(), ParserMode::Modsecurity, None, &flag, |p| {
            if p.lines_read >= CHECKPOINT_EVENTS * 2 {
                flag.store(true, Ordering::Relaxed);
            }
        }),
        Err(EngineError::ImportPaused)
    ));
    let checkpoint = e.pending_import().unwrap().unwrap();
    assert_eq!(checkpoint.events, CHECKPOINT_EVENTS);
    let summary = e
        .import_resumable(
            input.path(),
            ParserMode::Text,
            None,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    assert_eq!(summary.total_events, 10_005);
    assert_eq!(summary.total_lines, 20_010);
    let mut reference = Engine::new().unwrap();
    reference
        .import(
            input.path(),
            ParserMode::Modsecurity,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
    for id in [10_000, 10_001, 10_005] {
        assert_eq!(
            serde_json::to_value(e.detail(id).unwrap()).unwrap(),
            serde_json::to_value(reference.detail(id).unwrap()).unwrap()
        );
    }
}
