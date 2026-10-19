use chrono::DateTime;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParserMode {
    #[default]
    Auto,
    Apache,
    Nginx,
    JsonLines,
    Syslog,
    Iptables,
    Haproxy,
    Modsecurity,
    Text,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParseStatus {
    Parsed,
    Partial,
    #[default]
    Unparsed,
}

impl ParseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parsed => "parsed",
            Self::Partial => "partial",
            Self::Unparsed => "unparsed",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedFields {
    pub parser: String,
    pub parse_status: ParseStatus,
    pub timestamp: Option<String>,
    pub timestamp_ms: Option<i64>,
    pub source_ip: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub protocol: Option<String>,
    pub status: Option<i32>,
    pub response_bytes: Option<i64>,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub parse_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_fields: Option<serde_json::Map<String, serde_json::Value>>,
}

fn apache_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r#"^(?P<host>\S+) \S+ \S+ \[(?P<time>[^\]]+)\] "(?P<request>(?:\\.|[^"\\])*)" (?P<status>\d{3}) (?P<bytes>\d+|-)(?: "(?P<referrer>(?:\\.|[^"\\])*)" "(?P<agent>(?:\\.|[^"\\])*)")?\s*$"#)
            .expect("the built-in Apache pattern is valid")
    })
}

pub fn parse_line(line: &str, mode: ParserMode) -> ParsedFields {
    match mode {
        ParserMode::Syslog => return crate::security_formats::syslog(line),
        ParserMode::Iptables => return crate::security_formats::iptables(line),
        ParserMode::Haproxy => return crate::security_formats::haproxy(line),
        ParserMode::Modsecurity => return crate::security_formats::modsecurity(line),
        _ => {}
    }
    let mut fields = ParsedFields {
        parser: "text".into(),
        ..Default::default()
    };
    if mode == ParserMode::Text {
        fields.parse_status = ParseStatus::Parsed;
        return fields;
    }
    if mode == ParserMode::JsonLines
        || (mode == ParserMode::Auto && line.trim_start().starts_with('{'))
    {
        return parse_json(line);
    }
    let Some(captures) = apache_pattern().captures(line) else {
        if matches!(mode, ParserMode::Apache | ParserMode::Nginx) {
            fields.parser = "unparsed".into();
            fields.parse_error =
                Some("Line does not match the selected Common/Combined access format.".into());
        }
        return fields;
    };
    // Validate independently so a bad required field cannot erase valid evidence.
    fields.parser = if mode == ParserMode::Nginx {
        "nginx"
    } else {
        "apache"
    }
    .into();
    fields.source_ip = Some(captures["host"].into());
    fields.referrer = captures.name("referrer").map(|v| v.as_str().into());
    fields.user_agent = captures.name("agent").map(|v| v.as_str().into());
    let mut errors = Vec::new();
    match DateTime::parse_from_str(&captures["time"], "%d/%b/%Y:%H:%M:%S %z") {
        Ok(timestamp) => {
            fields.timestamp = Some(timestamp.to_rfc3339());
            fields.timestamp_ms = Some(timestamp.timestamp_millis());
        }
        Err(_) => errors.push("Invalid Apache timestamp or timezone."),
    }
    match captures["status"].parse::<i32>() {
        Ok(status) if (100..=599).contains(&status) => fields.status = Some(status),
        _ => errors.push("HTTP status must be between 100 and 599."),
    }
    if &captures["bytes"] != "-" {
        match captures["bytes"].parse::<i64>() {
            Ok(bytes) => fields.response_bytes = Some(bytes),
            Err(_) => errors.push("Response byte count is too large."),
        }
    }
    let request = &captures["request"];
    if request != "-" {
        let parts: Vec<_> = request.split_whitespace().collect();
        if parts.len() == 3 && parts[2].starts_with("HTTP/") {
            fields.method = Some(parts[0].into());
            fields.path = Some(parts[1].into());
            fields.protocol = Some(parts[2].into());
        } else {
            errors.push("Invalid Apache HTTP request field.");
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
    fields
}

/// JSON Lines objects retain every original field; normalization only uses the
/// documented aliases. No local timezone or epoch unit is guessed.
fn parse_json(line: &str) -> ParsedFields {
    let mut fields = ParsedFields {
        parser: "json_lines".into(),
        ..Default::default()
    };
    struct JsonObject {
        fields: serde_json::Map<String, serde_json::Value>,
        duplicate: bool,
    }
    impl<'de> Deserialize<'de> for JsonObject {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = JsonObject;
                fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str("a JSON object")
                }
                fn visit_map<M: serde::de::MapAccess<'de>>(
                    self,
                    mut map: M,
                ) -> Result<Self::Value, M::Error> {
                    let mut fields = serde_json::Map::new();
                    let mut duplicate = false;
                    while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                        duplicate |= fields.insert(key, value).is_some();
                    }
                    Ok(JsonObject { fields, duplicate })
                }
            }
            deserializer.deserialize_map(Visitor)
        }
    }
    let Ok(parsed) = serde_json::from_str::<JsonObject>(line) else {
        fields.parse_error = Some("Expected one valid JSON object per line.".into());
        return fields;
    };
    let object = parsed.fields;
    let mut errors = Vec::new();
    if parsed.duplicate {
        errors.push("Duplicate top-level JSON keys; extracted values use the last occurrence. Original raw values are retained.".into());
    }
    fn selected<'a>(
        object: &'a serde_json::Map<String, serde_json::Value>,
        keys: &[&str],
    ) -> Option<&'a serde_json::Value> {
        keys.iter()
            .find_map(|key| object.get(*key).filter(|v| !v.is_null()))
    }
    fn string(
        object: &serde_json::Map<String, serde_json::Value>,
        keys: &[&str],
        errors: &mut Vec<String>,
    ) -> Option<String> {
        selected(object, keys).and_then(|v| match v.as_str() {
            Some(value) => Some(value.into()),
            None => {
                errors.push(format!("{} must be a string.", keys[0]));
                None
            }
        })
    }
    fn integer(
        object: &serde_json::Map<String, serde_json::Value>,
        keys: &[&str],
        errors: &mut Vec<String>,
    ) -> Option<i64> {
        selected(object, keys).and_then(|v| {
            match v
                .as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            {
                Some(value) => Some(value),
                None => {
                    errors.push(format!(
                        "{} must be an integer in the supported range.",
                        keys[0]
                    ));
                    None
                }
            }
        })
    }
    fields.source_ip = string(
        &object,
        &["sourceIp", "source_ip", "remote_addr", "client_ip"],
        &mut errors,
    );
    fields.method = string(&object, &["method", "request_method"], &mut errors);
    fields.path = string(&object, &["path", "request_uri", "url"], &mut errors);
    fields.protocol = string(&object, &["protocol", "server_protocol"], &mut errors);
    fields.referrer = string(&object, &["referrer", "http_referer"], &mut errors);
    fields.user_agent = string(
        &object,
        &["userAgent", "user_agent", "http_user_agent"],
        &mut errors,
    );
    if let Some(status) = integer(&object, &["status"], &mut errors) {
        if (100..=599).contains(&status) {
            fields.status = Some(status as i32);
        } else {
            errors.push("HTTP status must be between 100 and 599.".into());
        }
    }
    if let Some(bytes) = integer(
        &object,
        &["responseBytes", "response_bytes", "body_bytes_sent"],
        &mut errors,
    ) {
        if bytes >= 0 {
            fields.response_bytes = Some(bytes);
        } else {
            errors.push("Response bytes cannot be negative.".into());
        }
    }
    let timestamp_keys = [
        "timestamp",
        "@timestamp",
        "time_iso8601",
        "time",
        "time_local",
    ];
    let text_time = string(&object, &timestamp_keys, &mut errors);
    if let Some(value) = text_time {
        let time = DateTime::parse_from_rfc3339(&value)
            .or_else(|_| DateTime::parse_from_str(&value, "%d/%b/%Y:%H:%M:%S %z"));
        match time {
            Ok(time) if time.timestamp_millis().abs_diff(0) <= 8_640_000_000_000_000 => {
                fields.timestamp = Some(time.to_rfc3339());
                fields.timestamp_ms = Some(time.timestamp_millis());
            }
            _ => errors.push(
                "Timestamp requires a valid RFC 3339 or access-log date with an explicit offset."
                    .into(),
            ),
        }
    } else if selected(&object, &timestamp_keys).is_none() {
        if let Some(ms) = integer(&object, &["timestamp_ms"], &mut errors) {
            match DateTime::from_timestamp_millis(ms)
                .filter(|_| ms.abs_diff(0) <= 8_640_000_000_000_000)
            {
                Some(time) => {
                    fields.timestamp = Some(time.to_rfc3339());
                    fields.timestamp_ms = Some(ms);
                }
                None => errors.push("timestamp_ms is outside the supported date range.".into()),
            }
        }
    }
    if fields.method.is_none() && fields.path.is_none() && fields.protocol.is_none() {
        if let Some(request) =
            string(&object, &["request"], &mut errors).filter(|request| request != "-")
        {
            let parts: Vec<_> = request.split_whitespace().collect();
            if parts.len() == 3 && parts[2].starts_with("HTTP/") {
                fields.method = Some(parts[0].into());
                fields.path = Some(parts[1].into());
                fields.protocol = Some(parts[2].into());
            } else {
                errors.push("Invalid JSON HTTP request field.".into());
            }
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
    fields.json_fields = Some(object);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMON: &str =
        r#"192.0.2.10 - - [10/Oct/2000:13:55:36 -0700] "GET /index.html HTTP/1.1" 200 2326"#;

    #[test]
    fn parses_common_and_preserves_timezone() {
        let fields = parse_line(COMMON, ParserMode::Auto);
        assert_eq!(fields.parser, "apache");
        assert_eq!(
            fields.timestamp.as_deref(),
            Some("2000-10-10T13:55:36-07:00")
        );
        assert_eq!(fields.timestamp_ms, Some(971211336000));
        assert_eq!(fields.path.as_deref(), Some("/index.html"));
        assert_eq!(fields.status, Some(200));
        assert_eq!(fields.response_bytes, Some(2326));
    }

    #[test]
    fn parses_combined_ipv6_and_escaped_agent() {
        let line = r#"2001:db8::1 - user [13/Oct/2026:10:00:00 +0200] "POST /login HTTP/2.0" 401 - "-" "Agent\"quoted""#;
        let fields = parse_line(line, ParserMode::Apache);
        assert_eq!(fields.source_ip.as_deref(), Some("2001:db8::1"));
        assert_eq!(fields.user_agent.as_deref(), Some(r#"Agent\"quoted"#));
        assert_eq!(fields.response_bytes, None);
        assert!(fields.parse_error.is_none());
    }

    #[test]
    fn keeps_unrecognized_text_and_marks_forced_parse_failure() {
        assert_eq!(
            parse_line("unstructured text", ParserMode::Auto).parser,
            "text"
        );
        assert!(parse_line("unstructured text", ParserMode::Apache)
            .parse_error
            .is_some());
        assert_eq!(parse_line(COMMON, ParserMode::Text).parser, "text");
    }

    #[test]
    fn rejects_ambiguous_or_invalid_dates_and_statuses() {
        assert!(parse_line(&COMMON.replace("-0700", ""), ParserMode::Apache)
            .parse_error
            .is_some());
        assert!(
            parse_line(&COMMON.replace("200 2326", "999 2326"), ParserMode::Auto)
                .parse_error
                .is_some()
        );
    }
    #[test]
    fn partial_retains_valid_fields_and_optional_absences_are_not_failures() {
        let bad = COMMON
            .replace("10/Oct/2000:13:55:36 -0700", "invalid date")
            .replace("200 2326", "999 2326");
        let fields = parse_line(&bad, ParserMode::Apache);
        assert_eq!(fields.parse_status, ParseStatus::Partial);
        assert_eq!(fields.source_ip.as_deref(), Some("192.0.2.10"));
        assert_eq!(fields.path.as_deref(), Some("/index.html"));
        assert_eq!(fields.response_bytes, Some(2326));
        assert_eq!(fields.timestamp, None);
        assert_eq!(fields.status, None);
        assert!(fields.parse_error.unwrap().contains("HTTP status"));
        let optional = parse_line(
            &COMMON
                .replace("GET /index.html HTTP/1.1", "-")
                .replace("2326", "-"),
            ParserMode::Apache,
        );
        assert_eq!(optional.parse_status, ParseStatus::Parsed);
        assert!(optional.parse_error.is_none());
        assert_eq!(optional.method, None);
        assert_eq!(optional.response_bytes, None);
        assert_eq!(optional.referrer, None);
        let malformed = parse_line(
            &COMMON.replace("GET /index.html HTTP/1.1", "GET /index.html"),
            ParserMode::Apache,
        );
        assert_eq!(malformed.parse_status, ParseStatus::Partial);
        assert_eq!(malformed.timestamp_ms, Some(971211336000));
        assert_eq!(malformed.method, None);
        assert_eq!(
            parse_line(&COMMON[..COMMON.len() / 2], ParserMode::Apache).parse_status,
            ParseStatus::Unparsed
        );
        assert_eq!(
            parse_line("unexpected format", ParserMode::Auto).parse_status,
            ParseStatus::Unparsed
        );
        assert_eq!(
            parse_line("raw text", ParserMode::Text).parse_status,
            ParseStatus::Parsed
        );
    }
}
