//! Deliberately scoped security-log readers. Extra attributes use the same
//! parameterized storage as JSON scalars; original bytes remain authoritative.
use crate::{ParseStatus, ParsedFields};
use chrono::{DateTime, NaiveDateTime};
use regex::Regex;
use serde_json::{Map, Value};
use std::{collections::HashSet, net::IpAddr, sync::OnceLock};

fn pattern(cell: &'static OnceLock<Regex>, expression: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(expression).expect("valid built-in security format"))
}
fn empty(parser: &str) -> ParsedFields {
    ParsedFields {
        parser: parser.into(),
        ..Default::default()
    }
}
fn finish(
    mut fields: ParsedFields,
    attributes: Map<String, Value>,
    errors: Vec<String>,
) -> ParsedFields {
    fields.parse_status = if errors.is_empty() {
        ParseStatus::Parsed
    } else {
        ParseStatus::Partial
    };
    fields.parse_error = (!errors.is_empty()).then(|| errors.join(" "));
    fields.json_fields = Some(attributes);
    fields
}
fn mismatch(parser: &str, message: &str) -> ParsedFields {
    ParsedFields {
        parse_error: Some(message.into()),
        ..empty(parser)
    }
}
fn put(attributes: &mut Map<String, Value>, key: &str, value: &str) {
    if value != "-" {
        attributes.insert(key.into(), value.into());
    }
}
fn date(fields: &mut ParsedFields, value: &str, format: Option<&str>, errors: &mut Vec<String>) {
    if value == "-" {
        return;
    }
    let parsed = match format {
        Some(format) => DateTime::parse_from_str(value, format),
        None => DateTime::parse_from_rfc3339(value),
    };
    match parsed {
        Ok(time)
            if !value.ends_with("-00:00")
                && time.timestamp_millis().abs_diff(0) <= 8_640_000_000_000_000 =>
        {
            fields.timestamp = Some(time.to_rfc3339());
            fields.timestamp_ms = Some(time.timestamp_millis());
        }
        _ => {
            errors.push("Timestamp requires a valid date and an explicit, known UTC offset.".into())
        }
    }
}
fn ip(value: &str, name: &str, errors: &mut Vec<String>) -> Option<String> {
    match value.parse::<IpAddr>() {
        Ok(_) => Some(value.into()),
        Err(_) => {
            errors.push(format!("Invalid {name} IP address."));
            None
        }
    }
}
fn port(value: &str, name: &str, errors: &mut Vec<String>) {
    if value.parse::<u16>().is_err() {
        errors.push(format!("Invalid {name} port (0–65535 required)."));
    }
}
fn request(fields: &mut ParsedFields, value: &str, errors: &mut Vec<String>) {
    if value == "-" || value == "<BADREQ>" {
        return;
    }
    let parts: Vec<_> = value.split_whitespace().collect();
    if parts.len() == 3 && parts[2].starts_with("HTTP/") {
        fields.method = Some(parts[0].into());
        fields.path = Some(parts[1].into());
        fields.protocol = Some(parts[2].into());
    } else {
        errors.push("Invalid HTTP request line.".into());
    }
}
fn status(fields: &mut ParsedFields, value: &str, errors: &mut Vec<String>) {
    match value.parse::<i32>() {
        Ok(code) if (100..=599).contains(&code) => fields.status = Some(code),
        _ => errors.push("HTTP status must be between 100 and 599.".into()),
    }
}

/// RFC 5424 version 1 and a BSD file layout with HOST TAG[PID]: MSG.
/// Returns the payload separately so specialized readers do not scan headers.
fn envelope(line: &str) -> Option<(ParsedFields, &str)> {
    static MODERN: OnceLock<Regex> = OnceLock::new();
    static BSD: OnceLock<Regex> = OnceLock::new();
    let mut fields = empty("syslog");
    let mut attributes = Map::new();
    let mut errors = Vec::new();
    let modern = pattern(
        &MODERN,
        r"^<(?P<pri>[0-9]{1,3})>(?P<version>[0-9]{1,3}) (?P<time>\S+) (?P<host>\S+) (?P<app>\S+) (?P<pid>\S+) (?P<msgid>\S+) (?P<tail>.*)$",
    );
    let bsd = pattern(
        &BSD,
        r"^(?:<(?P<pri>[0-9]{1,3})>)?(?P<time>[A-Z][a-z]{2} +[0-9]{1,2} [0-9]{2}:[0-9]{2}:[0-9]{2}) (?P<host>\S+) (?P<app>[^\s:\[]+)(?:\[(?P<pid>[^\]]+)\])?: ?(?P<message>.*)$",
    );
    let (priority, payload) = if let Some(c) = modern.captures(line) {
        if &c["version"] != "1" {
            errors.push("Only Syslog version 1 is supported.".into());
        }
        for (name, limit) in [("host", 255), ("app", 48), ("pid", 128), ("msgid", 32)] {
            let value = &c[name];
            put(&mut attributes, &format!("syslog.{name}"), value);
            if value.len() > limit || !value.bytes().all(|b| (33..=126).contains(&b)) {
                errors.push(format!("Invalid Syslog {name} header."));
            }
        }
        put(&mut attributes, "syslog.timestamp", &c["time"]);
        static RFC_DATE: OnceLock<Regex> = OnceLock::new();
        let time = &c["time"];
        if time == "-" || pattern(&RFC_DATE, r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-5][0-9](?:\.[0-9]{1,6})?(?:Z|[+-][0-9]{2}:[0-9]{2})$").is_match(time) {
            date(&mut fields, time, None, &mut errors);
        } else {
            errors.push("Invalid RFC 5424 timestamp syntax (uppercase T/Z, seconds 00–59, at most six fractional digits).".into());
        }
        let tail = c.name("tail").unwrap().as_str();
        let payload = structured_data(tail, &mut attributes, &mut errors);
        (Some(c["pri"].parse::<u16>().unwrap()), payload)
    } else {
        let c = bsd.captures(line)?;
        put(&mut attributes, "syslog.timestamp", &c["time"]);
        put(&mut attributes, "syslog.host", &c["host"]);
        put(&mut attributes, "syslog.app", &c["app"]);
        if let Some(pid) = c.name("pid") {
            put(&mut attributes, "syslog.pid", pid.as_str());
        }
        // A fixed leap year is used only to validate month/day, never to produce an instant.
        if NaiveDateTime::parse_from_str(&format!("2000 {}", &c["time"]), "%Y %b %e %H:%M:%S")
            .is_err()
        {
            errors.push("Invalid BSD Syslog calendar date.".into());
        }
        errors.push("BSD Syslog has no year or UTC offset; timestamp is retained as an attribute and excluded from UTC filters.".into());
        (
            c.name("pri").map(|v| v.as_str().parse::<u16>().unwrap()),
            c.name("message").unwrap().as_str(),
        )
    };
    if let Some(priority) = priority {
        if priority <= 191 {
            attributes.insert("syslog.facility".into(), (priority / 8).into());
            attributes.insert("syslog.severity".into(), (priority % 8).into());
        } else {
            errors.push("Syslog priority must be between 0 and 191.".into());
        }
    }
    attributes.insert(
        "syslog.message".into(),
        payload.strip_prefix('\u{feff}').unwrap_or(payload).into(),
    );
    Some((finish(fields, attributes, errors), payload))
}
fn structured_data<'a>(
    tail: &'a str,
    attributes: &mut Map<String, Value>,
    errors: &mut Vec<String>,
) -> &'a str {
    if tail == "-" {
        return "";
    }
    if let Some(message) = tail.strip_prefix("- ") {
        return message;
    }
    static ELEMENT: OnceLock<Regex> = OnceLock::new();
    static PARAM: OnceLock<Regex> = OnceLock::new();
    let element = pattern(
        &ELEMENT,
        r#"^\[(?P<id>[^\s=\]"]{1,32})(?P<params>(?: [^\s=\]"]{1,32}="(?:\\["\\\]]|[^"\\\]])*")*)\]"#,
    );
    let parameter = pattern(
        &PARAM,
        r#" (?P<key>[^\s=\]"]+)="(?P<value>(?:\\["\\\]]|[^"\\\]])*)""#,
    );
    let mut rest = tail;
    let mut ids = HashSet::new();
    while rest.starts_with('[') {
        let Some(c) = element.captures(rest) else {
            break;
        };
        let id = &c["id"];
        if !ids.insert(id.to_owned()) {
            errors.push("Duplicate Syslog structured-data ID.".into());
        }
        if !id.bytes().all(|b| (33..=126).contains(&b)) {
            errors.push("Syslog structured-data names must be printable ASCII.".into());
        }
        let mut keys = HashSet::new();
        for p in parameter.captures_iter(&c["params"]) {
            let key = &p["key"];
            if !keys.insert(key.to_owned()) {
                errors.push("Duplicate Syslog parameter; last value used.".into());
            }
            if !key.bytes().all(|b| (33..=126).contains(&b)) {
                errors.push("Syslog parameter names must be printable ASCII.".into());
            }
            let mut value = String::new();
            let mut chars = p["value"].chars();
            while let Some(ch) = chars.next() {
                value.push(if ch == '\\' {
                    chars.next().unwrap()
                } else {
                    ch
                });
            }
            if attributes
                .insert(format!("syslog.sd.{id}.{key}"), value.into())
                .is_some()
            {
                errors.push(
                    "Syslog structured-data attribute-name collision; last value used.".into(),
                );
            }
        }
        rest = &rest[c.get(0).unwrap().end()..];
    }
    if rest == tail || (!rest.is_empty() && !rest.starts_with(' ')) {
        errors.push("Invalid or incomplete Syslog structured data.".into());
        return rest;
    }
    rest.strip_prefix(' ').unwrap_or(rest)
}
pub(crate) fn syslog(line: &str) -> ParsedFields {
    envelope(line).map(|(fields, _)| fields).unwrap_or_else(|| {
        mismatch(
            "syslog",
            "Expected RFC 5424 or BSD HOST TAG[PID]: MSG Syslog layout.",
        )
    })
}

pub(crate) fn iptables(line: &str) -> ParsedFields {
    let (mut fields, payload) = envelope(line).unwrap_or_else(|| (empty("iptables"), line));
    fields.parser = "iptables".into();
    let mut attributes = fields.json_fields.take().unwrap_or_default();
    let mut errors: Vec<String> = fields.parse_error.take().into_iter().collect();
    let mut values = Map::new();
    let mut prefix = Vec::new();
    for token in payload.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            if !key.is_empty()
                && key
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            {
                if values
                    .insert(key.into(), Value::String(value.into()))
                    .is_some()
                {
                    errors.push(format!("Duplicate firewall field {key}; last value used."));
                }
                continue;
            }
        }
        if values.is_empty() {
            prefix.push(token);
        }
    }
    if !["IN", "OUT", "SRC", "DST", "PROTO"]
        .iter()
        .all(|key| values.contains_key(*key))
    {
        return mismatch(
            "iptables",
            "Expected Netfilter LOG fields IN, OUT, SRC, DST and PROTO.",
        );
    }
    fields.source_ip = ip(values["SRC"].as_str().unwrap(), "source", &mut errors);
    ip(values["DST"].as_str().unwrap(), "destination", &mut errors);
    let protocol = values["PROTO"].as_str().unwrap();
    fields.protocol = Some(protocol.into());
    if protocol.is_empty() {
        errors.push("Firewall protocol is empty.".into());
    }
    for key in ["SPT", "DPT"] {
        if let Some(value) = values.get(key) {
            port(value.as_str().unwrap(), key, &mut errors);
        }
    }
    for key in ["LEN", "TTL", "ID", "WINDOW"] {
        if let Some(value) = values.get(key) {
            if value.as_str().unwrap().parse::<u64>().is_err() {
                errors.push(format!("Invalid nonnegative firewall {key}."));
            }
        }
    }
    put(&mut attributes, "firewall.prefix", &prefix.join(" "));
    for (key, value) in values {
        attributes.insert(format!("firewall.{key}"), value);
    }
    finish(fields, attributes, errors)
}

pub(crate) fn haproxy(line: &str) -> ParsedFields {
    let (envelope_fields, payload) = envelope(line).unwrap_or_else(|| (empty("haproxy"), line));
    static HTTP: OnceLock<Regex> = OnceLock::new();
    let Some(c) = pattern(&HTTP, r#"^(?P<ip>\S+):(?P<port>[0-9]+) \[(?P<time>[^\]]+)\] (?P<frontend>\S+) (?P<backend>\S+)/(?P<server>\S+) (?P<timers>\S+) (?P<status>\S+) (?P<bytes>\S+) (?P<request_cookie>\S+) (?P<response_cookie>\S+) (?P<termination>\S+) (?P<connections>\S+) (?P<queues>\S+)(?: \{[^}]*\}){0,2} "(?P<request>(?:\\.|[^"\\])*)"$"#).captures(payload) else {
        return mismatch("haproxy", "Expected the standard HAProxy HTTP log layout (option httplog).");
    };
    let mut fields = empty("haproxy");
    let mut attributes = envelope_fields.json_fields.unwrap_or_default();
    // Envelope date diagnostics remain relevant; an explicit envelope instant is
    // the emission time, not the request time, so it is never substituted.
    let mut errors: Vec<String> = envelope_fields.parse_error.into_iter().collect();
    fields.source_ip = ip(c["ip"].trim_matches(['[', ']']), "client", &mut errors);
    port(&c["port"], "client", &mut errors);
    for name in [
        "port",
        "time",
        "frontend",
        "backend",
        "server",
        "timers",
        "request_cookie",
        "response_cookie",
        "termination",
        "connections",
        "queues",
    ] {
        put(&mut attributes, &format!("haproxy.{name}"), &c[name]);
    }
    // Standard %tr has no UTC offset. An explicit offset variant is accepted.
    if DateTime::parse_from_str(&c["time"], "%d/%b/%Y:%H:%M:%S%.f %z").is_ok() {
        date(
            &mut fields,
            &c["time"],
            Some("%d/%b/%Y:%H:%M:%S%.f %z"),
            &mut errors,
        );
    } else {
        if NaiveDateTime::parse_from_str(&c["time"], "%d/%b/%Y:%H:%M:%S%.f").is_err() {
            errors.push("Invalid HAProxy request date.".into());
        }
        errors.push(
            "HAProxy request date has no explicit UTC offset; excluded from UTC filters.".into(),
        );
    }
    put(&mut attributes, "haproxy.status", &c["status"]);
    if &c["status"] != "0" {
        status(&mut fields, &c["status"], &mut errors);
    }
    if let Some(lower_bound) = c["bytes"].strip_prefix('+') {
        put(&mut attributes, "haproxy.bytes_lower_bound", lower_bound);
        if lower_bound.parse::<u64>().is_err() {
            errors.push("Invalid HAProxy byte-count lower bound.".into());
        }
        errors.push(
            "HAProxy logasap byte count is a lower bound; no exact response byte count extracted."
                .into(),
        );
    } else {
        match c["bytes"].parse::<i64>() {
            Ok(bytes) if bytes >= 0 => fields.response_bytes = Some(bytes),
            _ => errors.push("Invalid HAProxy response byte count.".into()),
        }
    }
    for (name, count, minimum) in [("timers", 5, -1), ("connections", 5, 0), ("queues", 2, 0)] {
        let parts: Vec<_> = c[name].split('/').collect();
        if parts.len() != count
            || parts.iter().any(|p| {
                p.trim_start_matches('+')
                    .parse::<i64>()
                    .map_or(true, |n| n < minimum)
            })
        {
            errors.push(format!("Invalid HAProxy {name}."));
        }
    }
    request(&mut fields, &c["request"], &mut errors);
    finish(fields, attributes, errors)
}

/// Only native ModSecurity 2.x boundaries are recognized. Matching tokens are
/// required to close a transaction; a fresh A boundary is a resynchronization point.
pub(crate) fn audit_boundary(line: &str) -> Option<(&str, char)> {
    let body = line.strip_prefix("--")?.strip_suffix("--")?;
    let (id, part) = body.rsplit_once('-')?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric()) || part.len() != 1 {
        return None;
    }
    let part = part.chars().next()?;
    part.is_ascii_uppercase().then_some((id, part))
}
pub(crate) fn modsecurity(record: &str) -> ParsedFields {
    let mut lines = record.lines();
    let Some((boundary, 'A')) = lines.next().and_then(audit_boundary) else {
        return mismatch(
            "modsecurity",
            "Expected a native ModSecurity 2.x --boundary-A-- transaction.",
        );
    };
    let mut fields = empty("modsecurity");
    let mut attributes = Map::new();
    let mut errors = Vec::new();
    put(&mut attributes, "waf.boundary", boundary);
    let mut sections: std::collections::BTreeMap<char, Vec<&str>> = Default::default();
    sections.insert('A', Vec::new());
    let mut current = 'A';
    let mut closed = false;
    let mut accept_section = true;
    for line in lines {
        if let Some((id, part)) = audit_boundary(line) {
            accept_section = id == boundary && !sections.contains_key(&part);
            if id != boundary {
                errors.push(
                    "Mismatched ModSecurity transaction boundary; section extraction skipped."
                        .into(),
                );
            } else if sections.contains_key(&part) {
                errors.push(format!(
                    "Duplicate ModSecurity section {part}; first section retained."
                ));
            }
            if part == 'Z' && id == boundary {
                closed = true;
                break;
            }
            current = part;
            if accept_section {
                sections.entry(part).or_default();
            }
        } else if accept_section {
            sections.entry(current).or_default().push(line);
        }
    }
    if !closed {
        errors.push("Incomplete ModSecurity transaction: missing matching Z terminator.".into());
    }
    static HEADER: OnceLock<Regex> = OnceLock::new();
    let header = sections[&'A'].first().copied().unwrap_or("");
    if let Some(c) = pattern(
        &HEADER,
        r"^\[(?P<time>[^\]]+)\] (?P<id>\S+) (?P<src>\S+) (?P<spt>\S+) (?P<dst>\S+) (?P<dpt>\S+)$",
    )
    .captures(header)
    {
        date(
            &mut fields,
            &c["time"],
            Some("%d/%b/%Y:%H:%M:%S %z"),
            &mut errors,
        );
        fields.source_ip = ip(&c["src"], "source", &mut errors);
        ip(&c["dst"], "destination", &mut errors);
        port(&c["spt"], "source", &mut errors);
        port(&c["dpt"], "destination", &mut errors);
        for key in ["id", "dst", "spt", "dpt"] {
            put(&mut attributes, &format!("waf.{key}"), &c[key]);
        }
    } else {
        errors.push("Invalid ModSecurity A header.".into());
    }
    if let Some(request_lines) = sections.get(&'B') {
        request(
            &mut fields,
            request_lines.first().copied().unwrap_or(""),
            &mut errors,
        );
        for line in request_lines.iter().skip(1) {
            if let Some((key, value)) = line.split_once(':') {
                match key.to_ascii_lowercase().as_str() {
                    "user-agent" => fields.user_agent = Some(value.trim().into()),
                    "referer" => fields.referrer = Some(value.trim().into()),
                    _ => {}
                }
            }
        }
    }
    if let Some(response_lines) = sections.get(&'F') {
        let parts: Vec<_> = response_lines
            .first()
            .copied()
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if parts.len() >= 2 && parts[0].starts_with("HTTP/") {
            status(&mut fields, parts[1], &mut errors);
        } else {
            errors.push("Invalid ModSecurity F response status line.".into());
        }
    }
    if let Some(trailer) = sections.get(&'H') {
        let messages: Vec<_> = trailer
            .iter()
            .filter_map(|line| line.strip_prefix("Message: "))
            .map(|v| Value::String(v.into()))
            .collect();
        attributes.insert("waf.messages".into(), messages.into());
        for line in trailer {
            if let Some(action) = line.strip_prefix("Action: ") {
                put(&mut attributes, "waf.action", action);
            }
        }
    }
    finish(fields, attributes, errors)
}
