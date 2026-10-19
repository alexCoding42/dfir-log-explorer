use crate::{
    parse_line, query::statistics, ParseStatus, ParsedFields, ParserMode, Query, Statistics,
};
use duckdb::{params, params_from_iter, types::Value, Config, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tempfile::TempDir;
use thiserror::Error;

pub const MAX_LINE_BYTES: usize = 1024 * 1024;
pub const PAGE_SIZE: u32 = 500;

// List responses are previews, including extracted strings. The full fields
// are returned only by detail(), so a long URL/agent cannot inflate a page IPC.
const LIST_COLUMNS: &str = "id, line_number, byte_offset, substring(message, 1, 240), parser,
    timestamp, timestamp_ms, substring(source_ip, 1, 160), substring(method, 1, 32),
    substring(path, 1, 240), substring(protocol, 1, 32), status, response_bytes,
    substring(referrer, 1, 240), substring(user_agent, 1, 240), substring(parse_error, 1, 512), parse_status, source_id, NULL AS json_fields";

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Unable to read the selected file or local session storage.")]
    Io(#[from] std::io::Error),
    #[error("The local analysis database could not complete the operation.")]
    Database(#[from] duckdb::Error),
    #[error("Import cancelled. No events from this import were kept.")]
    Cancelled,
    #[error(
        "A line exceeds the 1 MiB safety limit. Import was rolled back; the source is unchanged."
    )]
    LineTooLong,
    #[error("A multiline event exceeds the 1 MiB safety limit. Import was rolled back; the source is unchanged.")]
    RecordTooLong,
    #[error("The source changed during import. Import was rolled back; retry with a stable copy.")]
    SourceChanged,
    #[error("Select a regular file, not a directory or device.")]
    NotRegularFile,
    #[error("This version supports uncompressed UTF-8 text logs. Compressed, UTF-16, and binary formats need a dedicated reader.")]
    UnsupportedFormat,
    #[error("Invalid query: use a 1–500 limit, nonnegative offset, values up to 4096 characters, and a valid increasing time range.")]
    InvalidQuery,
    #[error("The selected event does not exist in this session.")]
    EventNotFound,
    #[error("The selected file is not a supported DFIR case (schema version 1 required).")]
    UnsupportedCase,
    #[error("A case already exists at this location. Choose a new filename; existing files are never overwritten.")]
    CaseExists,
    #[error("This case is already saved. Open it by its current filename.")]
    AlreadySaved,
    #[error("The case connection is unavailable after reopening local storage. The original case file is retained.")]
    CaseUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub bytes_read: u64,
    pub total_bytes: u64,
    pub lines_read: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSummary {
    pub id: u64,
    pub name: String,
    pub provenance: String,
    pub parser_mode: ParserMode,
    pub partial_events: u64,
    pub unparsed_events: u64,
    pub total_bytes: u64,
    pub total_events: u64,
    #[serde(default)]
    pub total_lines: u64,
    pub parsed_events: u64,
    pub text_events: u64,
    pub warning_events: u64,
    pub sha256: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseSummary {
    pub name: String,
    pub path: Option<String>,
    pub sources: Vec<SourceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRow {
    pub source_id: u64,
    pub id: u64,
    pub line_number: u64,
    pub byte_offset: u64,
    pub preview: String,
    #[serde(flatten)]
    pub fields: ParsedFields,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDetail {
    #[serde(flatten)]
    pub event: EventRow,
    pub end_line_number: u64,
    pub raw: String,
    pub raw_hex: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryPage {
    pub statistics: Statistics,
    pub rows: Vec<EventRow>,
    pub total: u64,
    pub offset: u64,
    pub limit: u32,
    pub elapsed_ms: u64,
}

/// One local, disk-backed database per case. A single owner
/// serializes access; the desktop layer runs all operations off the UI thread.
type StatisticsCache = std::collections::VecDeque<(String, Vec<Value>, Statistics)>;

pub struct Engine {
    statistics_cache: std::cell::RefCell<StatisticsCache>,
    connection: Option<Connection>,
    _storage: Option<TempDir>,
    database_path: PathBuf,
    case_name: String,
    has_json_attributes: bool,
}

impl Engine {
    pub fn new() -> Result<Self, EngineError> {
        let storage = tempfile::Builder::new()
            .prefix("dfir-log-explorer-")
            .tempdir()?;
        let path = storage.path().join("session.duckdb");
        let connection = Self::connect(&path)?;
        connection.execute_batch(
            "BEGIN;
             CREATE TABLE case_meta (schema_version INTEGER, name VARCHAR);
             INSERT INTO case_meta VALUES (1, 'Temporary case');
             CREATE TABLE sources (id BIGINT PRIMARY KEY, summary VARCHAR);
             CREATE TABLE event_attributes (event_id BIGINT, key VARCHAR, value VARCHAR);
             CREATE TABLE events (
                id BIGINT, line_number BIGINT, byte_offset BIGINT, raw BLOB,
                message VARCHAR, parser VARCHAR, timestamp VARCHAR, timestamp_ms BIGINT,
                source_ip VARCHAR, method VARCHAR, path VARCHAR, protocol VARCHAR,
                status INTEGER, response_bytes BIGINT, referrer VARCHAR, user_agent VARCHAR,
                parse_error VARCHAR, parse_status VARCHAR, source_id BIGINT, json_fields VARCHAR
             ); COMMIT;",
        )?;
        Ok(Self {
            statistics_cache: std::cell::RefCell::new(StatisticsCache::new()),
            connection: Some(connection),
            _storage: Some(storage),
            database_path: path,
            case_name: "Temporary case".into(),
            has_json_attributes: false,
        })
    }

    fn connection(&self) -> Result<&Connection, EngineError> {
        self.connection.as_ref().ok_or(EngineError::CaseUnavailable)
    }

    fn connect(path: &Path) -> Result<Connection, EngineError> {
        let config = Config::default()
            .enable_external_access(false)?
            .max_memory("512MB")?
            .threads(2)?;
        let connection = Connection::open_with_flags(path, config)?;
        connection.execute_batch(
            "SET autoinstall_known_extensions = false;
            SET autoload_known_extensions = false; SET allow_community_extensions = false;
            SET lock_configuration = true;",
        )?;
        Ok(connection)
    }

    /// Open only an existing case. Never initialize or migrate an unknown file.
    pub fn open_case(path: &Path) -> Result<Self, EngineError> {
        if !std::fs::metadata(path)?.is_file() {
            return Err(EngineError::NotRegularFile);
        }
        let connection = Self::connect(path)?;
        let (version, name): (i32, String) = connection
            .query_row("SELECT schema_version, name FROM case_meta", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|_| EngineError::UnsupportedCase)?;
        if version != 1 {
            return Err(EngineError::UnsupportedCase);
        }
        // Check required columns before replacing the active engine.
        connection
            .prepare("SELECT id, summary FROM sources")
            .map_err(|_| EngineError::UnsupportedCase)?;
        connection
            .prepare(&format!(
                "SELECT {LIST_COLUMNS}, json_fields, message, raw FROM events"
            ))
            .map_err(|_| EngineError::UnsupportedCase)?;
        connection
            .prepare("SELECT event_id, key, value FROM event_attributes")
            .map_err(|_| EngineError::UnsupportedCase)?;
        let has_json_attributes = connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM event_attributes LIMIT 1)",
            [],
            |row| row.get(0),
        )?;
        let engine = Self {
            statistics_cache: std::cell::RefCell::new(StatisticsCache::new()),
            connection: Some(connection),
            _storage: None,
            database_path: path.to_path_buf(),
            case_name: name,
            has_json_attributes,
        };
        engine.case_summary()?;
        Ok(engine)
    }

    /// Promote the complete temporary case, including raw bytes and provenance.
    /// Existing files are never overwritten. Subsequent imports commit to disk.
    pub fn save_case(&mut self, path: &Path) -> Result<CaseSummary, EngineError> {
        if self._storage.is_none() {
            return Err(EngineError::AlreadySaved);
        }
        if self.connection.is_none() {
            self.connection = Some(Self::connect(&self.database_path)?);
        }
        self.connection()?.execute_batch("CHECKPOINT;")?;
        if path.try_exists()? {
            return Err(EngineError::CaseExists);
        }
        let parent = path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut destination = tempfile::NamedTempFile::new_in(parent)?;
        // DuckDB holds an exclusive database-file handle on Windows. A
        // checkpoint alone does not permit a second reader, so release it for
        // the snapshot, then restore the current case before propagating errors.
        drop(self.connection.take());
        let copied = (|| -> Result<(), EngineError> {
            std::io::copy(
                &mut File::open(&self.database_path)?,
                destination.as_file_mut(),
            )?;
            destination.as_file().sync_all()?;
            destination.persist_noclobber(path).map_err(|error| {
                if error.error.kind() == std::io::ErrorKind::AlreadyExists {
                    EngineError::CaseExists
                } else {
                    EngineError::Io(error.error)
                }
            })?;
            Ok(())
        })();
        self.connection = Some(Self::connect(&self.database_path)?);
        copied?;
        let mut saved = Self::open_case(path)?;
        saved.case_name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        saved
            .connection()?
            .execute("UPDATE case_meta SET name = ?", params![saved.case_name])?;
        let summary = saved.case_summary()?;
        *self = saved;
        Ok(summary)
    }

    pub fn case_summary(&self) -> Result<CaseSummary, EngineError> {
        let mut statement = self
            .connection()?
            .prepare("SELECT summary FROM sources ORDER BY id")?;
        let serialized = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let sources = serialized
            .iter()
            .map(|value| serde_json::from_str(value).map_err(|_| EngineError::UnsupportedCase))
            .collect::<Result<Vec<SourceSummary>, _>>()?;
        Ok(CaseSummary {
            name: self.case_name.clone(),
            path: if self._storage.is_none() {
                Some(self.database_path.to_string_lossy().into_owned())
            } else {
                None
            },
            sources,
        })
    }

    /// Append a source atomically. Raw source bytes, including terminators and invalid
    /// UTF-8, are retained. Cancellation/error preserves the previous session.
    pub fn import(
        &mut self,
        path: &Path,
        mode: ParserMode,
        cancelled: &AtomicBool,
        progress: impl FnMut(ImportProgress),
    ) -> Result<SourceSummary, EngineError> {
        self.import_with_identity(path, mode, cancelled, progress, None)
    }

    pub fn import_with_identity(
        &mut self,
        path: &Path,
        mode: ParserMode,
        cancelled: &AtomicBool,
        mut progress: impl FnMut(ImportProgress),
        identity: Option<(&str, &str)>,
    ) -> Result<SourceSummary, EngineError> {
        if !std::fs::metadata(path)?.is_file() {
            return Err(EngineError::NotRegularFile);
        }
        let file = File::open(path)?;
        let before = file.metadata()?;
        if !before.is_file() {
            return Err(EngineError::NotRegularFile);
        }
        let total_bytes = before.len();
        let started = Instant::now();
        let mut last_progress = Instant::now();
        let mut reader = BufReader::with_capacity(256 * 1024, file);
        let signature = reader.fill_buf()?;
        if signature.starts_with(b"\x1f\x8b")
            || signature.starts_with(b"PK\x03\x04")
            || signature.starts_with(b"ElfFile\0")
            || signature.starts_with(b"\xff\xfe")
            || signature.starts_with(b"\xfe\xff")
        {
            return Err(EngineError::UnsupportedFormat);
        }
        let mut has_attributes = false;
        let mut buffer = Vec::with_capacity(4096);
        let mut pending = None;
        let mut events = 0u64;
        let mut hash = Sha256::new();
        let (
            mut bytes_read,
            mut lines,
            mut parsed,
            mut text,
            mut warnings,
            mut partial,
            mut unparsed,
        ) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);

        let source_id: i64 = self.connection()?.query_row(
            "SELECT coalesce(max(id), 0) + 1 FROM sources",
            [],
            |row| row.get(0),
        )?;
        let first_id: i64 =
            self.connection()?
                .query_row("SELECT coalesce(max(id), 0) FROM events", [], |row| {
                    row.get(0)
                })?;
        self.connection()?.execute_batch("BEGIN TRANSACTION;")?;
        let result = (|| {
            let mut appender = self.connection()?.appender("events")?;
            let mut attributes = self.connection()?.appender("event_attributes")?;
            progress(ImportProgress {
                bytes_read: 0,
                total_bytes,
                lines_read: 0,
                elapsed_ms: 0,
            });
            loop {
                if cancelled.load(Ordering::Relaxed) {
                    return Err(EngineError::Cancelled);
                }
                crate::records::read_record(
                    &mut reader,
                    mode,
                    &mut buffer,
                    &mut pending,
                    cancelled,
                )?;
                let size = buffer.len();
                if size == 0 {
                    break;
                }
                let first_line = lines + 1;
                lines += crate::records::physical_lines(&buffer);
                events += 1;
                hash.update(&buffer);
                let display_bytes = buffer.strip_suffix(b"\n").unwrap_or(&buffer);
                let display_bytes = display_bytes.strip_suffix(b"\r").unwrap_or(display_bytes);
                let message = String::from_utf8_lossy(display_bytes);
                let mut fields = parse_line(&message, mode);
                if std::str::from_utf8(display_bytes).is_err() {
                    let warning = "Invalid UTF-8; display uses replacement characters. Original bytes are preserved.";
                    fields.parse_error = Some(match fields.parse_error {
                        Some(error) => format!("{error} {warning}"),
                        None => warning.into(),
                    });
                }
                parsed += u64::from(fields.parse_status == ParseStatus::Parsed);
                partial += u64::from(fields.parse_status == ParseStatus::Partial);
                unparsed += u64::from(fields.parse_status == ParseStatus::Unparsed);
                text += u64::from(fields.parser == "text");
                warnings += u64::from(fields.parse_error.is_some());
                let json_fields = fields
                    .json_fields
                    .as_ref()
                    .map(|object| serde_json::Value::Object(object.clone()).to_string());
                if let Some(object) = &fields.json_fields {
                    for (key, value) in object {
                        let scalar = match value {
                            serde_json::Value::String(value) => Some(value.clone()),
                            serde_json::Value::Number(_) | serde_json::Value::Bool(_) => {
                                Some(value.to_string())
                            }
                            _ => None,
                        };
                        if let Some(value) = scalar {
                            has_attributes = true;
                            attributes.append_row(params![first_id + events as i64, key, value])?;
                        }
                    }
                }
                appender.append_row(params![
                    first_id + events as i64,
                    first_line as i64,
                    bytes_read as i64,
                    buffer.as_slice(),
                    message.as_ref(),
                    fields.parser,
                    fields.timestamp,
                    fields.timestamp_ms,
                    fields.source_ip,
                    fields.method,
                    fields.path,
                    fields.protocol,
                    fields.status,
                    fields.response_bytes,
                    fields.referrer,
                    fields.user_agent,
                    fields.parse_error,
                    fields.parse_status.as_str(),
                    source_id,
                    json_fields,
                ])?;
                bytes_read += size as u64;
                if last_progress.elapsed() >= Duration::from_millis(100) {
                    progress(ImportProgress {
                        bytes_read,
                        total_bytes,
                        lines_read: lines,
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    });
                    last_progress = Instant::now();
                }
            }
            appender.flush()?;
            drop(appender);
            attributes.flush()?;
            drop(attributes);
            if cancelled.load(Ordering::Relaxed) {
                return Err(EngineError::Cancelled);
            }
            let after = reader.get_ref().metadata()?;
            if bytes_read != total_bytes
                || after.len() != total_bytes
                || before.modified().ok() != after.modified().ok()
            {
                return Err(EngineError::SourceChanged);
            }
            let summary = SourceSummary {
                id: source_id as u64,
                provenance: identity
                    .map(|(_, provenance)| provenance.to_string())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned()),
                parser_mode: mode,
                partial_events: partial,
                unparsed_events: unparsed,
                name: identity
                    .map(|(name, _)| name.to_string())
                    .unwrap_or_else(|| {
                        path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    }),
                total_bytes,
                total_events: events,
                total_lines: lines,
                parsed_events: parsed,
                text_events: text,
                warning_events: warnings,
                sha256: format!("{:x}", hash.finalize()),
                elapsed_ms: started.elapsed().as_millis() as u64,
            };
            let serialized =
                serde_json::to_string(&summary).map_err(|_| EngineError::UnsupportedCase)?;
            self.connection()?.execute(
                "INSERT INTO sources VALUES (?, ?)",
                params![source_id, serialized],
            )?;
            self.connection()?.execute_batch("COMMIT;")?;
            Ok(summary)
        })();
        let summary = match result {
            Ok(summary) => summary,
            Err(error) => {
                self.connection()?.execute_batch("ROLLBACK;")?;
                return Err(error);
            }
        };
        self.has_json_attributes |= has_attributes;
        self.statistics_cache.borrow_mut().clear();
        progress(ImportProgress {
            bytes_read,
            total_bytes,
            lines_read: lines,
            elapsed_ms: started.elapsed().as_millis() as u64,
        });
        Ok(summary)
    }

    pub fn query(&self, query: &Query) -> Result<QueryPage, EngineError> {
        let started = Instant::now();
        let (predicate, values) = query.predicate_with_json(self.has_json_attributes)?;
        let cached = self
            .statistics_cache
            .borrow()
            .iter()
            .find(|(p, v, _)| p == &predicate && v == &values)
            .map(|(_, _, s)| s.clone());
        let statistics = match cached {
            Some(stats) => stats,
            None => {
                let stats = statistics(self.connection()?, &predicate, &values)?;
                let mut cache = self.statistics_cache.borrow_mut();
                if cache.len() == 4 {
                    cache.pop_front();
                }
                cache.push_back((predicate.clone(), values.clone(), stats.clone()));
                stats
            }
        };
        let total = statistics.total;
        let rows = if query.is_original_order() {
            let mut statement = self.connection()?.prepare(&format!(
                "SELECT {LIST_COLUMNS} FROM events WHERE id > ? ORDER BY id LIMIT ?"
            ))?;
            statement
                .query_map(params![query.offset as i64, query.limit], event_row)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let direction = if query.descending { "DESC" } else { "ASC" };
            let mut statement = self.connection()?.prepare(&format!(
                "SELECT {LIST_COLUMNS} FROM events WHERE {predicate} ORDER BY {} {direction} NULLS LAST, id ASC LIMIT ? OFFSET ?",
                query.sort_by.sql()
            ))?;
            let mut page_values = values;
            page_values.push(Value::BigInt(query.limit as i64));
            page_values.push(Value::BigInt(query.offset as i64));
            statement
                .query_map(params_from_iter(page_values.iter()), event_row)?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(QueryPage {
            statistics,
            rows,
            total,
            offset: query.offset,
            limit: query.limit,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }

    pub fn detail(&self, id: u64) -> Result<EventDetail, EngineError> {
        if id > i64::MAX as u64 {
            return Err(EngineError::EventNotFound);
        }
        let mut statement = self.connection()?.prepare(
            "SELECT id, line_number, byte_offset, substring(message, 1, 240), parser,
                    timestamp, timestamp_ms, source_ip, method, path, protocol, status,
                    response_bytes, referrer, user_agent, parse_error, parse_status, source_id, json_fields, message, raw
             FROM events WHERE id = ?",
        )?;
        let mut rows = statement.query(params![id as i64])?;
        let row = rows.next()?.ok_or(EngineError::EventNotFound)?;
        let raw_bytes: Vec<u8> = row.get(20)?;
        Ok(EventDetail {
            end_line_number: row.get::<_, i64>(1)? as u64
                + crate::records::physical_lines(&raw_bytes).saturating_sub(1),
            event: event_row(row)?,
            raw: row.get(19)?,
            raw_hex: std::str::from_utf8(&raw_bytes)
                .err()
                .map(|_| raw_bytes.iter().map(|byte| format!("{byte:02x}")).collect()),
        })
    }
}

fn event_row(row: &duckdb::Row<'_>) -> duckdb::Result<EventRow> {
    Ok(EventRow {
        source_id: row.get::<_, i64>(17)? as u64,
        id: row.get::<_, i64>(0)? as u64,
        line_number: row.get::<_, i64>(1)? as u64,
        byte_offset: row.get::<_, i64>(2)? as u64,
        preview: row.get(3)?,
        fields: ParsedFields {
            parser: row.get(4)?,
            parse_status: match row.get::<_, String>(16)?.as_str() {
                "parsed" => ParseStatus::Parsed,
                "partial" => ParseStatus::Partial,
                _ => ParseStatus::Unparsed,
            },
            timestamp: row.get(5)?,
            timestamp_ms: row.get(6)?,
            source_ip: row.get(7)?,
            method: row.get(8)?,
            path: row.get(9)?,
            protocol: row.get(10)?,
            status: row.get(11)?,
            response_bytes: row.get(12)?,
            referrer: row.get(13)?,
            user_agent: row.get(14)?,
            parse_error: row.get(15)?,
            json_fields: row
                .get::<_, Option<String>>(18)?
                .and_then(|value| serde_json::from_str(&value).ok()),
        },
    })
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn database_disables_external_access_and_extension_downloads() {
        let engine = Engine::new().unwrap();
        for setting in [
            "enable_external_access",
            "autoinstall_known_extensions",
            "autoload_known_extensions",
            "allow_community_extensions",
        ] {
            let enabled: bool = engine
                .connection()
                .unwrap()
                .query_row(&format!("SELECT current_setting('{setting}')"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert!(!enabled, "{setting} must be disabled");
        }
        let source = tempfile::NamedTempFile::new().unwrap();
        assert!(engine
            .connection()
            .unwrap()
            .query_row(
                "SELECT content FROM read_text(?)",
                params![source.path().to_str().unwrap()],
                |row| row.get::<_, String>(0)
            )
            .is_err());
        assert!(engine
            .connection()
            .unwrap()
            .execute_batch("SET enable_external_access = true")
            .is_err());
    }
}
