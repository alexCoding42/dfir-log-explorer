use crate::{EngineError, ParseStatus, ParserMode};
use duckdb::{params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SearchField {
    #[default]
    All,
    Raw,
    Timestamp,
    SourceIp,
    Method,
    Path,
    Status,
    ResponseBytes,
    Protocol,
    Referrer,
    UserAgent,
}
impl SearchField {
    pub fn sql(self) -> &'static str {
        match self {
            Self::All => "concat_ws(' ', message, timestamp, source_ip, method, path, status, response_bytes, protocol, referrer, user_agent)",
            Self::Raw => "message", Self::Timestamp => "timestamp", Self::SourceIp => "source_ip",
            Self::Method => "method", Self::Path => "path", Self::Status => "CAST(status AS VARCHAR)",
            Self::ResponseBytes => "CAST(response_bytes AS VARCHAR)", Self::Protocol => "protocol",
            Self::Referrer => "referrer", Self::UserAgent => "user_agent",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SortField {
    SourceId,
    #[default]
    LineNumber,
    Timestamp,
    SourceIp,
    Method,
    Path,
    Status,
    ResponseBytes,
    Referrer,
    UserAgent,
    ParseStatus,
    Preview,
}
impl SortField {
    pub fn sql(self) -> &'static str {
        match self {
            Self::SourceId => "source_id",
            Self::LineNumber => "id",
            Self::Timestamp => "timestamp_ms",
            Self::SourceIp => "source_ip",
            Self::Method => "method",
            Self::Path => "path",
            Self::Status => "status",
            Self::ResponseBytes => "response_bytes",
            Self::Referrer => "referrer",
            Self::UserAgent => "user_agent",
            Self::ParseStatus => "parse_status",
            Self::Preview => "message",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldFilter {
    pub field: SearchField,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonFilter {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Query {
    pub source_id: Option<u64>,
    pub parser: Option<ParserMode>,
    pub has_timestamp: Option<bool>,
    pub json_filter: Option<JsonFilter>,
    pub search: String,
    pub only_warnings: bool,
    pub offset: u64,
    pub limit: u32,
    pub search_field: SearchField,
    pub parse_status: Option<ParseStatus>,
    pub field_filter: Option<FieldFilter>,
    pub time_from: Option<i64>,
    pub time_to: Option<i64>,
    pub sort_by: SortField,
    pub descending: bool,
}
impl Query {
    pub fn predicate(&self) -> Result<(String, Vec<Value>), EngineError> {
        self.predicate_with_json(true)
    }

    pub(crate) fn predicate_with_json(
        &self,
        include_json: bool,
    ) -> Result<(String, Vec<Value>), EngineError> {
        if self
            .source_id
            .is_some_and(|id| id == 0 || id > i64::MAX as u64)
            || self.parser == Some(ParserMode::Auto)
            || self.json_filter.as_ref().is_some_and(|f| {
                f.key.is_empty() || f.key.chars().count() > 4096 || f.value.chars().count() > 4096
            })
            || self.limit == 0
            || self.limit > crate::PAGE_SIZE
            || self.offset > i64::MAX as u64
            || self.search.chars().count() > 4096
            || self
                .field_filter
                .as_ref()
                .is_some_and(|f| f.value.chars().count() > 4096)
            || self
                .time_from
                .is_some_and(|t| t.abs_diff(0) > 8_640_000_000_000_000)
            || self
                .time_to
                .is_some_and(|t| t.abs_diff(0) > 8_640_000_000_000_000)
            || matches!((self.time_from, self.time_to), (Some(a), Some(b)) if a >= b)
        {
            return Err(EngineError::InvalidQuery);
        }
        let mut clauses = vec!["true".to_string()];
        let mut values = Vec::new();
        if !self.search.is_empty() {
            if self.search_field == SearchField::All {
                if include_json {
                    clauses.push("(contains(lower(message), lower(?)) OR contains(lower(timestamp), lower(?)) OR EXISTS (SELECT 1 FROM event_attributes a WHERE a.event_id = events.id AND contains(lower(a.value), lower(?))))".into());
                    values.push(Value::Text(self.search.clone()));
                } else {
                    clauses.push("(contains(lower(message), lower(?)) OR contains(lower(timestamp), lower(?)))".into());
                }
                values.push(Value::Text(self.search.clone()));
            } else {
                clauses.push(format!(
                    "contains(lower(coalesce({}, '')), lower(?))",
                    self.search_field.sql()
                ));
            }
            values.push(Value::Text(self.search.clone()));
        }
        if let Some(id) = self.source_id {
            clauses.push("source_id = ?".into());
            values.push(Value::BigInt(id as i64));
        }
        if let Some(parser) = self.parser {
            let name = match parser {
                ParserMode::Apache => "apache",
                ParserMode::Nginx => "nginx",
                ParserMode::JsonLines => "json_lines",
                ParserMode::Text => "text",
                ParserMode::Syslog => "syslog",
                ParserMode::Iptables => "iptables",
                ParserMode::Haproxy => "haproxy",
                ParserMode::Modsecurity => "modsecurity",
                ParserMode::Auto => unreachable!(),
            };
            clauses.push("parser = ?".into());
            values.push(Value::Text(name.into()));
        }
        if let Some(present) = self.has_timestamp {
            clauses.push(
                if present {
                    "timestamp_ms IS NOT NULL"
                } else {
                    "timestamp_ms IS NULL"
                }
                .into(),
            );
        }
        if let Some(filter) = &self.json_filter {
            clauses.push("EXISTS (SELECT 1 FROM event_attributes a WHERE a.event_id = events.id AND a.key = ? AND a.value = ?)".into());
            values.push(Value::Text(filter.key.clone()));
            values.push(Value::Text(filter.value.clone()));
        }
        if self.only_warnings {
            clauses.push("parse_error IS NOT NULL".into());
        }
        if let Some(status) = self.parse_status {
            clauses.push("parse_status = ?".into());
            values.push(Value::Text(status.as_str().into()));
        }
        if let Some(filter) = &self.field_filter {
            clauses.push(format!("{} = ?", filter.field.sql()));
            values.push(Value::Text(filter.value.clone()));
        }
        if let Some(from) = self.time_from {
            clauses.push("timestamp_ms >= ?".into());
            values.push(Value::BigInt(from));
        }
        if let Some(to) = self.time_to {
            clauses.push("timestamp_ms < ?".into());
            values.push(Value::BigInt(to));
        }
        Ok((clauses.join(" AND "), values))
    }
    pub fn is_original_order(&self) -> bool {
        self.source_id.is_none()
            && self.parser.is_none()
            && self.has_timestamp.is_none()
            && self.json_filter.is_none()
            && self.search.is_empty()
            && !self.only_warnings
            && self.parse_status.is_none()
            && self.field_filter.is_none()
            && self.time_from.is_none()
            && self.time_to.is_none()
            && self.sort_by == SortField::LineNumber
            && !self.descending
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    pub total: u64,
    pub parsed: u64,
    pub partial: u64,
    pub unparsed: u64,
    pub warnings: u64,
    pub missing_timestamp: u64,
    pub total_requests: u64,
    pub unique_ips: u64,
    pub responses: [u64; 4],
    pub timeline: Vec<TimeBucket>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeBucket {
    pub start_ms: i64,
    pub end_ms: i64,
    pub parsed: u64,
    pub partial: u64,
    pub unparsed: u64,
    pub responses: [u64; 4],
}

pub(crate) fn statistics(
    connection: &Connection,
    predicate: &str,
    values: &[Value],
) -> Result<Statistics, EngineError> {
    let (mut stats, min, max) = connection.query_row(&format!(
        "SELECT count(*), count(*) FILTER (WHERE parse_status = 'parsed'),
         count(*) FILTER (WHERE parse_status = 'partial'), count(*) FILTER (WHERE parse_status = 'unparsed'),
         count(parse_error), count(*) FILTER (WHERE timestamp_ms IS NULL),
         count(*) FILTER (WHERE method IS NOT NULL), count(DISTINCT source_ip),
         count(*) FILTER (WHERE status BETWEEN 200 AND 299), count(*) FILTER (WHERE status BETWEEN 300 AND 399),
         count(*) FILTER (WHERE status BETWEEN 400 AND 499), count(*) FILTER (WHERE status BETWEEN 500 AND 599),
         min(timestamp_ms), max(timestamp_ms) FROM events WHERE {predicate}"
    ), params_from_iter(values.iter()), |row| Ok((Statistics {
        total: row.get::<_, i64>(0)? as u64, parsed: row.get::<_, i64>(1)? as u64,
        partial: row.get::<_, i64>(2)? as u64, unparsed: row.get::<_, i64>(3)? as u64,
        warnings: row.get::<_, i64>(4)? as u64, missing_timestamp: row.get::<_, i64>(5)? as u64,
        total_requests: row.get::<_, i64>(6)? as u64, unique_ips: row.get::<_, i64>(7)? as u64,
        responses: [row.get::<_, i64>(8)? as u64, row.get::<_, i64>(9)? as u64, row.get::<_, i64>(10)? as u64, row.get::<_, i64>(11)? as u64],
        timeline: Vec::new(),
    }, row.get::<_, Option<i64>>(12)?, row.get::<_, Option<i64>>(13)?)))?;
    if let (Some(min), Some(max)) = (min, max) {
        // Bounded aggregates only: never transfer individual events to draw a chart.
        let width = ((max - min + 32) / 32).max(1000);
        let count = ((max - min) / width + 1) as usize;
        stats.timeline = (0..count)
            .map(|i| TimeBucket {
                start_ms: min + i as i64 * width,
                end_ms: min + (i as i64 + 1) * width,
                parsed: 0,
                partial: 0,
                unparsed: 0,
                responses: [0; 4],
            })
            .collect();
        let mut statement = connection.prepare(&format!(
            "SELECT CAST(floor((timestamp_ms - {min}) / {width}) AS BIGINT) AS bucket,
             count(*) FILTER (WHERE parse_status = 'parsed'), count(*) FILTER (WHERE parse_status = 'partial'),
             count(*) FILTER (WHERE parse_status = 'unparsed'), count(*) FILTER (WHERE status BETWEEN 200 AND 299),
             count(*) FILTER (WHERE status BETWEEN 300 AND 399), count(*) FILTER (WHERE status BETWEEN 400 AND 499),
             count(*) FILTER (WHERE status BETWEEN 500 AND 599)
             FROM events WHERE {predicate} AND timestamp_ms IS NOT NULL GROUP BY bucket ORDER BY bucket"
        ))?;
        let mut rows = statement.query(params_from_iter(values.iter()))?;
        while let Some(row) = rows.next()? {
            let index = row.get::<_, i64>(0)? as usize;
            if let Some(bucket) = stats.timeline.get_mut(index) {
                bucket.parsed = row.get::<_, i64>(1)? as u64;
                bucket.partial = row.get::<_, i64>(2)? as u64;
                bucket.unparsed = row.get::<_, i64>(3)? as u64;
                bucket.responses = [
                    row.get::<_, i64>(4)? as u64,
                    row.get::<_, i64>(5)? as u64,
                    row.get::<_, i64>(6)? as u64,
                    row.get::<_, i64>(7)? as u64,
                ];
            }
        }
    }
    Ok(stats)
}
