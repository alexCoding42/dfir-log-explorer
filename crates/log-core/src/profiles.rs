//! Versioned, declarative single-line readers. No executable profile code.
use crate::{Engine, EngineError, ParseStatus, ParsedFields, MAX_LINE_BYTES};
use chrono::DateTime;
use duckdb::params;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_PROFILE_PATTERN: usize = 4096;
pub const MAX_PREVIEW_BYTES: usize = 65_536;
pub const MAX_PREVIEW_LINES: usize = 100;
const MAX_PROFILE_PAYLOAD: usize = 16_384;
const MAX_PROFILE_VERSIONS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileDefinition {
    pub version: u32,
    pub name: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileSelection {
    pub id: u64,
    pub revision: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileRevision {
    pub id: u64,
    pub revision: u32,
    pub definition: ProfileDefinition,
    pub sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePreviewRow {
    pub line_number: u64,
    pub byte_offset: u64,
    pub raw: String,
    pub fields: ParsedFields,
}

pub(crate) struct CompiledProfile(Regex);
impl ProfileDefinition {
    pub(crate) fn compile(&self) -> Result<CompiledProfile, EngineError> {
        if self.version != 1 {
            return Err(EngineError::UnsupportedProfile);
        }
        if self.name.trim().is_empty()
            || self.name != self.name.trim()
            || self.name.chars().count() > 120
            || self.name.chars().any(char::is_control)
            || self.pattern.len() > MAX_PROFILE_PATTERN
            || self.pattern.is_empty()
        {
            return Err(EngineError::InvalidProfile(
                "Use a name of 1–120 characters and a pattern of 1–4096 bytes.".into(),
            ));
        }
        let regex = RegexBuilder::new(&format!(r"\A(?:{})\z", self.pattern))
            .size_limit(1_048_576)
            .dfa_size_limit(1_048_576)
            .nest_limit(32)
            .build()
            .map_err(|_| EngineError::InvalidProfile("Invalid or oversized Rust regular expression. Lookaround and backreferences are unsupported.".into()))?;
        let names: Vec<_> = regex.capture_names().flatten().collect();
        if names.is_empty()
            || names.len() > 32
            || regex.captures_len() != names.len() + 1
            || names.iter().any(|name| {
                name.len() > 32 || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
        {
            return Err(EngineError::InvalidProfile("Use 1–32 named captures with ASCII names of at most 32 bytes; use (?:...) for noncapturing groups.".into()));
        }
        Ok(CompiledProfile(regex))
    }
    fn digest(&self) -> Result<String, EngineError> {
        let bytes = serde_json::to_vec(self).map_err(|_| EngineError::UnsupportedProfile)?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}
impl ProfileRevision {
    pub(crate) fn validate(&self) -> Result<(), EngineError> {
        if self.id == 0
            || self.id > 9_007_199_254_740_991
            || self.revision == 0
            || self.revision > 20
            || self.sha256 != self.definition.digest()?
        {
            return Err(EngineError::UnsupportedProfile);
        }
        self.definition.compile()?;
        Ok(())
    }
}
impl CompiledProfile {
    pub(crate) fn parse(&self, line: &str) -> ParsedFields {
        let mut fields = ParsedFields {
            parser: "custom".into(),
            ..Default::default()
        };
        let Some(captures) = self.0.captures(line) else {
            fields.parse_error =
                Some("Line does not match the selected profile's full-line pattern.".into());
            return fields;
        };
        let mut values = serde_json::Map::new();
        for name in self.0.capture_names().flatten() {
            if let Some(value) = captures.name(name) {
                values.insert(name.into(), value.as_str().into());
            }
        }
        let text = |name: &str| values.get(name).and_then(|v| v.as_str()).map(str::to_owned);
        let mut errors = Vec::new();
        if let Some(value) = text("timestamp") {
            match DateTime::parse_from_rfc3339(&value) {
                Ok(time) if time.timestamp_millis().abs_diff(0) <= 8_640_000_000_000_000 => {
                    fields.timestamp = Some(time.to_rfc3339());
                    fields.timestamp_ms = Some(time.timestamp_millis());
                }
                _ => errors.push("timestamp requires RFC 3339 with an explicit UTC offset."),
            }
        }
        fields.source_ip = text("source_ip");
        fields.method = text("method");
        fields.path = text("path");
        fields.protocol = text("protocol");
        fields.referrer = text("referrer");
        fields.user_agent = text("user_agent");
        if let Some(value) = text("status") {
            match value.parse::<i32>() {
                Ok(value) if (100..=599).contains(&value) => fields.status = Some(value),
                _ => errors.push("status must be an HTTP code between 100 and 599."),
            }
        }
        if let Some(value) = text("response_bytes") {
            match value.parse::<i64>() {
                Ok(value) if value >= 0 => fields.response_bytes = Some(value),
                _ => errors.push("response_bytes must be a nonnegative 64-bit integer."),
            }
        }
        fields.parse_status = if errors.is_empty() {
            ParseStatus::Parsed
        } else {
            ParseStatus::Partial
        };
        if !errors.is_empty() {
            fields.parse_error = Some(errors.join(" "));
        }
        fields.json_fields = Some(values);
        fields
    }
}

/// Reject oversized previews rather than silently sampling/truncating evidence.
pub fn preview_profile(
    definition: &ProfileDefinition,
    sample: &str,
) -> Result<Vec<ProfilePreviewRow>, EngineError> {
    if sample.len() > MAX_PREVIEW_BYTES {
        return Err(EngineError::ProfilePreviewLimit);
    }
    let lines: Vec<_> = sample.split_inclusive('\n').collect();
    if lines.len() > MAX_PREVIEW_LINES {
        return Err(EngineError::ProfilePreviewLimit);
    }
    let compiled = definition.compile()?;
    let mut offset = 0;
    Ok(lines
        .into_iter()
        .enumerate()
        .map(|(i, raw)| {
            let line = raw.strip_suffix('\n').unwrap_or(raw);
            let line = line.strip_suffix('\r').unwrap_or(line);
            let fields = compiled.parse(line);
            let row = ProfilePreviewRow {
                line_number: i as u64 + 1,
                byte_offset: offset,
                raw: raw.into(),
                fields,
            };
            offset += raw.len() as u64;
            row
        })
        .collect())
}

impl Engine {
    fn has_profiles(&self) -> Result<bool, EngineError> {
        Ok(self.connection()?.query_row(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'main' AND table_name = 'parsing_profiles')",
            [], |row| row.get(0),
        )?)
    }
    /// Reading legacy cases never creates an extension table.
    pub fn parsing_profiles(&self) -> Result<Vec<ProfileRevision>, EngineError> {
        if !self.has_profiles()? {
            return Ok(Vec::new());
        }
        let mut statement = self.connection()?.prepare(
            "SELECT profile_id, revision, substring(payload, 1, 16385) FROM parsing_profiles ORDER BY profile_id, revision LIMIT 201"
        ).map_err(|_| EngineError::UnsupportedProfile)?;
        let stored = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| EngineError::UnsupportedProfile)?;
        if stored.len() > MAX_PROFILE_VERSIONS {
            return Err(EngineError::UnsupportedProfile);
        }
        let mut profiles: Vec<ProfileRevision> = Vec::new();
        for (id, revision, payload) in stored {
            if payload.len() > MAX_PROFILE_PAYLOAD {
                return Err(EngineError::UnsupportedProfile);
            }
            let profile: ProfileRevision =
                serde_json::from_str(&payload).map_err(|_| EngineError::UnsupportedProfile)?;
            if id != profile.id
                || revision != profile.revision
                || revision != profiles.iter().filter(|p| p.id == id).count() as u32 + 1
            {
                return Err(EngineError::UnsupportedProfile);
            }
            profile
                .validate()
                .map_err(|_| EngineError::UnsupportedProfile)?;
            if (revision == 1 && profiles.iter().filter(|p| p.revision == 1).count() >= 50)
                || profiles.iter().any(|p| {
                    p.id != id
                        && p.definition.name.to_lowercase()
                            == profile.definition.name.to_lowercase()
                })
            {
                return Err(EngineError::UnsupportedProfile);
            }
            profiles.push(profile);
        }
        Ok(profiles)
    }
    pub fn profile_revision(
        &self,
        selection: ProfileSelection,
    ) -> Result<ProfileRevision, EngineError> {
        self.parsing_profiles()?
            .into_iter()
            .find(|p| p.id == selection.id && p.revision == selection.revision)
            .ok_or(EngineError::UnsupportedProfile)
    }
    /// A stale editor cannot silently create a revision on top of somebody else's edit.
    pub fn put_parsing_profile(
        &self,
        previous: Option<ProfileSelection>,
        mut definition: ProfileDefinition,
    ) -> Result<ProfileRevision, EngineError> {
        definition.name = definition.name.trim().into();
        definition.compile()?;
        let existing = self.parsing_profiles()?;
        if existing.len() >= MAX_PROFILE_VERSIONS {
            return Err(EngineError::ProfileStorageLimit);
        }
        let (id, revision) = if let Some(previous) = previous {
            let latest = existing
                .iter()
                .filter(|p| p.id == previous.id)
                .map(|p| p.revision)
                .max();
            if latest != Some(previous.revision) {
                return Err(EngineError::ProfileConflict);
            }
            if previous.revision >= 20 {
                return Err(EngineError::ProfileStorageLimit);
            }
            (previous.id, previous.revision + 1)
        } else {
            let count = existing.iter().filter(|p| p.revision == 1).count();
            if count >= 50 {
                return Err(EngineError::ProfileStorageLimit);
            }
            (existing.iter().map(|p| p.id).max().unwrap_or(0) + 1, 1)
        };
        if existing.iter().any(|p| {
            p.id != id && p.definition.name.to_lowercase() == definition.name.to_lowercase()
        }) {
            return Err(EngineError::InvalidProfile(
                "Choose a name not used by another profile's history.".into(),
            ));
        }
        let profile = ProfileRevision {
            id,
            revision,
            sha256: definition.digest()?,
            definition,
        };
        profile.validate()?;
        let payload =
            serde_json::to_string(&profile).map_err(|_| EngineError::UnsupportedProfile)?;
        if payload.len() > MAX_PROFILE_PAYLOAD {
            return Err(EngineError::ProfileStorageLimit);
        }
        let connection = self.connection()?;
        connection.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), EngineError> {
            connection.execute_batch("CREATE TABLE IF NOT EXISTS parsing_profiles (profile_id BIGINT, revision INTEGER, payload VARCHAR, PRIMARY KEY (profile_id, revision))")?;
            connection.execute(
                "INSERT INTO parsing_profiles VALUES (?, ?, ?)",
                params![id as i64, revision, payload],
            )?;
            connection.execute_batch("COMMIT")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = connection.execute_batch("ROLLBACK");
        }
        result?;
        Ok(profile)
    }
}

// A profile does not change the physical-record limit of ordinary ingestion.
const _: () = assert!(MAX_PREVIEW_BYTES < MAX_LINE_BYTES);
