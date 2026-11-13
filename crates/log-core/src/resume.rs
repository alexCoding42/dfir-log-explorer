use crate::{CompressionInfo, Engine, EngineError, ParserMode, ProfileRevision};
use duckdb::params;
use serde::{Deserialize, Serialize};

pub const CHECKPOINT_EVENTS: u64 = 10_000;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingImport {
    pub version: u32,
    pub path: String,
    pub mode: ParserMode,
    pub profile: Option<ProfileRevision>,
    pub compression: Option<CompressionInfo>,
    pub sha256: String,
    pub total_bytes: u64,
    pub source_id: i64,
    pub first_id: i64,
    pub bytes_read: u64,
    pub lines: u64,
    pub events: u64,
    pub parsed: u64,
    pub partial: u64,
    pub unparsed: u64,
    pub text: u64,
    pub warnings: u64,
}
impl Engine {
    pub fn pending_import(&self) -> Result<Option<PendingImport>, EngineError> {
        let exists: bool = self.connection()?.query_row("SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'ingestion_checkpoint')", [], |r| r.get(0))?;
        if !exists {
            return Ok(None);
        }
        let mut statement = self
            .connection()?
            .prepare("SELECT state FROM ingestion_checkpoint")?;
        let states = statement
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        if states.is_empty() {
            return Ok(None);
        }
        if states.len() != 1 {
            return Err(EngineError::InvalidCheckpoint);
        }
        let p: PendingImport =
            serde_json::from_str(&states[0]).map_err(|_| EngineError::InvalidCheckpoint)?;
        if p.version != 1
            || p.bytes_read > p.total_bytes
            || p.source_id < 1
            || p.first_id < 0
            || p.parsed
                .checked_add(p.partial)
                .and_then(|n| n.checked_add(p.unparsed))
                != Some(p.events)
            || p.lines < p.events
        {
            return Err(EngineError::InvalidCheckpoint);
        }
        if p.sha256.len() != 64
            || !p.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || (p.mode == ParserMode::Custom) != p.profile.is_some()
        {
            return Err(EngineError::InvalidCheckpoint);
        }
        let (last_event, next_source): (i64, i64) = self.connection()?.query_row(
            "SELECT (SELECT coalesce(max(id),0) FROM events), (SELECT coalesce(max(id),0)+1 FROM sources)", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
        if last_event != p.first_id || next_source != p.source_id {
            return Err(EngineError::InvalidCheckpoint);
        }
        if let Some(profile) = &p.profile {
            profile.validate()?;
        }
        let (rows, unique_ids, first, last, wrong_source): (i64, i64, i64, i64, i64) = self.connection()?.query_row(
            "SELECT count(*), count(DISTINCT id), coalesce(min(id),0), coalesce(max(id),0), count(*) FILTER (WHERE source_id <> ?) FROM ingestion_events",
            params![p.source_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?;
        if rows as u64 != p.events || unique_ids != rows || wrong_source != 0 {
            return Err(EngineError::InvalidCheckpoint);
        }
        if p.events == 0 {
            if p.bytes_read != 0 || p.lines != 0 {
                return Err(EngineError::InvalidCheckpoint);
            }
        } else {
            let expected_last = i64::try_from(p.events)
                .ok()
                .and_then(|events| p.first_id.checked_add(events))
                .ok_or(EngineError::InvalidCheckpoint)?;
            if p.first_id.checked_add(1) != Some(first) || last != expected_last {
                return Err(EngineError::InvalidCheckpoint);
            }
            let (line, offset, raw): (i64, i64, Vec<u8>) = self.connection()?.query_row(
                "SELECT line_number, byte_offset, raw FROM ingestion_events WHERE id = ?",
                params![last],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            if offset < 0
                || line < 1
                || offset as u64 + raw.len() as u64 != p.bytes_read
                || line as u64 + crate::records::physical_lines(&raw).saturating_sub(1) != p.lines
            {
                return Err(EngineError::InvalidCheckpoint);
            }
        }
        Ok(Some(p))
    }
    pub fn discard_pending_import(&self) -> Result<(), EngineError> {
        self.connection()?.execute_batch("BEGIN; DROP TABLE IF EXISTS ingestion_events; DROP TABLE IF EXISTS ingestion_attributes; DROP TABLE IF EXISTS ingestion_checkpoint; COMMIT;")?;
        Ok(())
    }
    pub(crate) fn write_checkpoint(&self, state: &PendingImport) -> Result<(), EngineError> {
        let serialized =
            serde_json::to_string(state).map_err(|_| EngineError::InvalidCheckpoint)?;
        self.connection()?
            .execute("DELETE FROM ingestion_checkpoint", [])?;
        self.connection()?.execute(
            "INSERT INTO ingestion_checkpoint VALUES (?)",
            params![serialized],
        )?;
        Ok(())
    }
}
