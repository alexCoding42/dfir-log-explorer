use crate::{engine::event_row, records::physical_lines, Engine, EngineError, EventDetail, Query};
use duckdb::params_from_iter;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufWriter, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Csv,
    JsonLines,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub events_written: u64,
}

impl Engine {
    /// Export all matching results, ignoring pagination. No external SQL access.
    #[allow(clippy::too_many_arguments)]
    pub fn export(
        &self,
        query: &Query,
        format: ExportFormat,
        include_raw: bool,
        path: &Path,
        cancelled: &AtomicBool,
        mut progress: impl FnMut(ExportProgress),
    ) -> Result<u64, EngineError> {
        let (predicate, values) = query.predicate_with_json(self.has_json_attributes)?;
        let sources = self.case_summary()?.sources;
        let direction = if query.descending { "DESC" } else { "ASC" };
        let mut statement = self.connection()?.prepare(&format!(
            "SELECT id, line_number, byte_offset, '' AS preview, parser, timestamp, timestamp_ms, source_ip, method, path, protocol, status, response_bytes, referrer, user_agent, parse_error, parse_status, source_id, json_fields, message, raw FROM events WHERE {predicate} ORDER BY {} {direction} NULLS LAST, id ASC", query.sort_by.sql()))?;
        let mut rows = statement.query(params_from_iter(values.iter()))?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        let mut writer = BufWriter::new(temporary.as_file_mut());
        let mut count = 0u64;
        progress(ExportProgress { events_written: 0 });
        // CSV keeps full structured metadata in JSON cells; quoting never changes values.
        if matches!(format, ExportFormat::Csv) {
            writer.write_all(b"schema_version,source_json,event_json\r\n")?;
        }
        while let Some(row) = rows.next()? {
            if cancelled.load(Ordering::Relaxed) {
                return Err(EngineError::ExportCancelled);
            }
            let bytes: Vec<u8> = row.get(20)?;
            let detail = EventDetail {
                end_line_number: row.get::<_, i64>(1)? as u64
                    + physical_lines(&bytes).saturating_sub(1),
                event: event_row(row)?,
                raw: row.get(19)?,
                raw_hex: std::str::from_utf8(&bytes)
                    .err()
                    .map(|_| bytes.iter().map(|b| format!("{b:02x}")).collect()),
            };
            let source = sources
                .iter()
                .find(|s| s.id == detail.event.source_id)
                .ok_or(EngineError::UnsupportedCase)?;
            let mut event =
                serde_json::to_value(&detail).map_err(|_| EngineError::UnsupportedCase)?;
            let object = event.as_object_mut().ok_or(EngineError::UnsupportedCase)?;
            object.remove("preview");
            if include_raw {
                // Preserve terminators too, unlike the inspector's display text.
                object.insert(
                    "raw".into(),
                    String::from_utf8_lossy(&bytes).into_owned().into(),
                );
            } else {
                object.remove("raw");
                object.remove("rawHex");
            }
            match format {
                ExportFormat::JsonLines => {
                    serde_json::to_writer(
                        &mut writer,
                        &serde_json::json!({"schemaVersion":1,"source":source,"event":event}),
                    )
                    .map_err(std::io::Error::other)?;
                    writer.write_all(b"\n")?;
                }
                ExportFormat::Csv => {
                    let source_json =
                        serde_json::to_string(source).map_err(std::io::Error::other)?;
                    let event_json =
                        serde_json::to_string(&event).map_err(std::io::Error::other)?;
                    writeln!(
                        writer,
                        "1,\"{}\",\"{}\"\r",
                        source_json.replace('"', "\"\""),
                        event_json.replace('"', "\"\"")
                    )?;
                }
            }
            count += 1;
            if count.is_multiple_of(500) {
                progress(ExportProgress {
                    events_written: count,
                });
            }
        }
        writer.flush()?;
        drop(writer);
        if cancelled.load(Ordering::Relaxed) {
            return Err(EngineError::ExportCancelled);
        }
        temporary.as_file().sync_all()?;
        temporary
            .persist_noclobber(path)
            .map_err(|e| EngineError::Io(e.error))?;
        progress(ExportProgress {
            events_written: count,
        });
        Ok(count)
    }
}
