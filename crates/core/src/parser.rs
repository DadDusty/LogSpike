//! Lightweight format detection.
//!
//! The viewer renders any plain-text file, but we make a best-effort attempt
//! to extract a log level and timestamp so the UI can colour rows and offer
//! "jump to time" without the user configuring anything.
//!
//! Detection is intentionally cheap (a couple of substring scans, no regex
//! compile per line). Custom format plugins can be layered on top later.

use serde::{Serialize, Deserialize};
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Plain,
    Json,
    CMTrace,
    Nginx,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedLine {
    pub level: Option<LogLevel>,
    /// Iso-8601-ish timestamp string lifted from the line, if found. Kept as
    /// text rather than parsed time to avoid pulling chrono into the hot path.
    pub timestamp: Option<String>,
    pub component: Option<String>,
    pub source: Option<String>,
    pub message: Option<String>,
}

/// Detect the file format from the first 20 lines.
pub fn detect_format(lines: &[&str]) -> FileFormat {
    let non_empty: Vec<&str> = lines.iter().filter(|l| !l.is_empty()).take(20).copied().collect();
    if non_empty.is_empty() {
        return FileFormat::Plain;
    }

    // CMTrace: contains <![LOG[
    if non_empty.iter().any(|l| l.contains("<![LOG[")) {
        return FileFormat::CMTrace;
    }

    // JSON: >= 80% of lines start with {
    let json_count = non_empty.iter().filter(|l| l.trim_start().starts_with('{')).count();
    if json_count as f64 / non_empty.len() as f64 >= 0.8 {
        return FileFormat::Json;
    }

    // Nginx combined log: matches pattern like "192.168.1.1 - - [27/Apr/2026:12:34:56 +0000]"
    let nginx_re = Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3} - .* \[").ok();
    if let Some(re) = nginx_re {
        let nginx_count = non_empty.iter().filter(|l| re.is_match(l)).count();
        if nginx_count as f64 / non_empty.len() as f64 >= 0.7 {
            return FileFormat::Nginx;
        }
    }

    FileFormat::Plain
}

/// Parse a single line with format-awareness. Returns `None` if nothing recognisable was found.
pub fn parse_with_format(line: &str, format: FileFormat) -> Option<ParsedLine> {
    match format {
        FileFormat::CMTrace => parse_cmtrace(line),
        FileFormat::Json => parse_json(line),
        FileFormat::Nginx => parse_nginx(line),
        FileFormat::Plain => parse_plain(line),
    }
}

/// Parse a single line using plain heuristics. Returns `None` if nothing recognisable was found.
pub fn parse(line: &str) -> Option<ParsedLine> {
    parse_plain(line)
}

fn parse_plain(line: &str) -> Option<ParsedLine> {
    let level = detect_level(line);
    let timestamp = detect_timestamp(line);
    // Always return Some with extracted message if we found level or timestamp
    // This ensures plain text logs are still searchable/displayable
    if level.is_some() || timestamp.is_some() {
        Some(ParsedLine {
            level,
            timestamp,
            component: None,
            source: None,
            message: Some(line.to_string()),
        })
    } else {
        None
    }
}

fn parse_cmtrace(line: &str) -> Option<ParsedLine> {
    // CMTrace format: <![LOG[message]LOG]!><time="14:38:19.048-120" date="04-01-2022" component="CcmExec" context="" type="1" thread="6320" file="entrypoint.cpp:3340">
    if !line.contains("<![LOG[") {
        return None;
    }

    let start = line.find("<![LOG[")?;
    let end = line.find("]LOG]!")?;
    let message = line[start + 7..end].to_string();

    // Extract attributes from the XML-like tag after ]LOG]!
    let mut level = None;
    let mut timestamp = None;
    let mut component = None;
    let mut source = None;

    // Extract type to determine level
    if let Some(type_start) = line.find("type=\"") {
        let type_end = line[type_start + 6..].find('"')?;
        let type_val = &line[type_start + 6..type_start + 6 + type_end];
        level = match type_val {
            "1" => Some(LogLevel::Info),
            "2" => Some(LogLevel::Warn),
            "3" => Some(LogLevel::Error),
            _ => Some(LogLevel::Info),
        };
    }

    // Extract component
    if let Some(comp_start) = line.find("component=\"") {
        let comp_end = line[comp_start + 11..].find('"')?;
        component = Some(line[comp_start + 11..comp_start + 11 + comp_end].to_string());
    }

    // Extract file for source
    if let Some(file_start) = line.find("file=\"") {
        let file_end = line[file_start + 6..].find('"')?;
        source = Some(line[file_start + 6..file_start + 6 + file_end].to_string());
    }

    // Extract date and time to form timestamp
    if let (Some(date_start), Some(time_start)) = (line.find("date=\""), line.find("time=\"")) {
        let date_end = line[date_start + 6..].find('"')?;
        let time_end = line[time_start + 6..].find('"')?;
        let date = &line[date_start + 6..date_start + 6 + date_end];
        let time = &line[time_start + 6..time_start + 6 + time_end];
        timestamp = Some(format!("{} {}", date, time));
    }

    Some(ParsedLine {
        level,
        timestamp,
        component,
        source,
        message: Some(message),
    })
}

fn parse_json(line: &str) -> Option<ParsedLine> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }

    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let obj = value.as_object()?;

    // Try various common field names
    let level = obj
        .get("level")
        .or_else(|| obj.get("severity"))
        .or_else(|| obj.get("lvl"))
        .and_then(|v| v.as_str())
        .and_then(detect_level_from_string);

    let timestamp = obj
        .get("timestamp")
        .or_else(|| obj.get("time"))
        .or_else(|| obj.get("ts"))
        .or_else(|| obj.get("@timestamp"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let component = obj
        .get("component")
        .or_else(|| obj.get("logger"))
        .or_else(|| obj.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source = obj
        .get("file")
        .or_else(|| obj.get("caller"))
        .or_else(|| obj.get("src"))
        .or_else(|| obj.get("location"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let message = obj
        .get("message")
        .or_else(|| obj.get("msg"))
        .or_else(|| obj.get("content"))
        .or_else(|| obj.get("text"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Only return if we found at least one field
    if level.is_some() || timestamp.is_some() || component.is_some() || source.is_some() || message.is_some() {
        Some(ParsedLine {
            level,
            timestamp,
            component,
            source,
            message,
        })
    } else {
        None
    }
}

fn parse_nginx(line: &str) -> Option<ParsedLine> {
    // Nginx combined log format: 192.168.1.1 - user [27/Apr/2026:12:34:56 +0000] "GET /path HTTP/1.1" 200 1234 "referer" "user-agent"
    let re = Regex::new(r#"^(\d+\.\d+\.\d+\.\d+) - .* \[([^\]]+)\] "([^"]*)" (\d+)"#).ok()?;
    let caps = re.captures(line)?;

    let source = caps.get(1).map(|m| m.as_str().to_string());
    let timestamp = caps.get(2).map(|m| m.as_str().to_string());
    let message = caps.get(3).map(|m| m.as_str().to_string());
    let status_str = caps.get(4).map(|m| m.as_str())?;
    let status: u16 = status_str.parse().ok()?;

    let level = if status >= 500 {
        Some(LogLevel::Error)
    } else if status >= 400 {
        Some(LogLevel::Warn)
    } else {
        Some(LogLevel::Info)
    };

    Some(ParsedLine {
        level,
        timestamp,
        component: None,
        source,
        message,
    })
}

fn detect_level_from_string(s: &str) -> Option<LogLevel> {
    let upper = s.to_uppercase();
    match upper.as_str() {
        "TRACE" => Some(LogLevel::Trace),
        "DEBUG" | "DBG" => Some(LogLevel::Debug),
        "INFO" => Some(LogLevel::Info),
        "WARN" | "WARNING" => Some(LogLevel::Warn),
        "ERROR" | "ERR" => Some(LogLevel::Error),
        "FATAL" | "CRITICAL" => Some(LogLevel::Fatal),
        _ => None,
    }
}

fn detect_level(line: &str) -> Option<LogLevel> {
    // Scan the first ~120 bytes only — log levels are always near the start.
    let head_end = line.len().min(120);
    let head = &line[..head_end];

    // Order matters: check longer/more-specific tokens first to avoid
    // "WARN" matching inside "WARNING" twice, etc.
    const NEEDLES: &[(&str, LogLevel)] = &[
        ("FATAL", LogLevel::Fatal),
        ("ERROR", LogLevel::Error),
        ("ERR ", LogLevel::Error),
        ("WARN", LogLevel::Warn),
        ("INFO", LogLevel::Info),
        ("DEBUG", LogLevel::Debug),
        ("DBG ", LogLevel::Debug),
        ("TRACE", LogLevel::Trace),
    ];

    for (needle, level) in NEEDLES {
        if contains_ascii_ci(head, needle) {
            return Some(*level);
        }
    }
    None
}

fn detect_timestamp(line: &str) -> Option<String> {
    // Look for an Iso-8601 style timestamp at the very start of the line.
    // We require a digit prefix to keep this dirt cheap.
    let bytes = line.as_bytes();
    if bytes.len() < 10 || !bytes[0].is_ascii_digit() {
        return None;
    }
    // Trim until the first whitespace; that gives us "2026-04-27T12:34:56.789Z"
    // or similar without committing to a specific format.
    let end = bytes
        .iter()
        .position(|b| b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    if end < 8 {
        return None;
    }
    let candidate = &line[..end];
    // Sanity check: must contain at least one ':' or '-' to look like a time.
    if !candidate.contains(':') && !candidate.contains('-') {
        return None;
    }
    Some(candidate.to_owned())
}

/// Case-insensitive ASCII substring check that does not allocate.
fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|w| w.iter().zip(needle.bytes()).all(|(a, b)| a.eq_ignore_ascii_case(&b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_levels() {
        assert_eq!(detect_level("2026-04-27T10:00:00 INFO server started"), Some(LogLevel::Info));
        assert_eq!(detect_level("ERROR: something blew up"), Some(LogLevel::Error));
        assert_eq!(detect_level("[warn] disk almost full"), Some(LogLevel::Warn));
        assert_eq!(detect_level("plain text line"), None);
    }

    #[test]
    fn extracts_iso_timestamp_prefix() {
        let p = parse("2026-04-27T12:34:56.789Z INFO ready").unwrap();
        assert_eq!(p.level, Some(LogLevel::Info));
        assert_eq!(p.timestamp.as_deref(), Some("2026-04-27T12:34:56.789Z"));
    }

    #[test]
    fn returns_none_for_unmarked_lines() {
        assert!(parse("just a log line with nothing useful").is_none());
    }

    #[test]
    fn detects_cmtrace_format() {
        let lines = vec![
            "<![LOG[Starting CCMEXEC service...]LOG]!><time=\"14:38:19.048-120\" date=\"04-01-2022\" component=\"CcmExec\" context=\"\" type=\"1\" thread=\"6320\" file=\"entrypoint.cpp:3340\">",
        ];
        assert_eq!(detect_format(&lines), FileFormat::CMTrace);
    }

    #[test]
    fn detects_json_format() {
        let lines = vec![
            r#"{"level":"info","msg":"server started","timestamp":"2026-04-27T10:00:00Z"}"#,
            r#"{"level":"info","msg":"ready","timestamp":"2026-04-27T10:00:01Z"}"#,
        ];
        assert_eq!(detect_format(&lines), FileFormat::Json);
    }

    #[test]
    fn detects_nginx_format() {
        let lines = vec![
            r#"192.168.1.1 - - [27/Apr/2026:12:34:56 +0000] "GET /index.html HTTP/1.1" 200 1234 "-" "Mozilla/5.0""#,
            r#"192.168.1.2 - - [27/Apr/2026:12:34:57 +0000] "POST /api/users HTTP/1.1" 201 567 "-" "curl""#,
        ];
        assert_eq!(detect_format(&lines), FileFormat::Nginx);
    }

    #[test]
    fn parses_cmtrace_line() {
        let line = r#"<![LOG[Starting CCMEXEC service...]LOG]!><time="14:38:19.048-120" date="04-01-2022" component="CcmExec" context="" type="1" thread="6320" file="entrypoint.cpp:3340">"#;
        let p = parse_with_format(line, FileFormat::CMTrace).unwrap();
        assert_eq!(p.level, Some(LogLevel::Info));
        assert_eq!(p.component.as_deref(), Some("CcmExec"));
        assert_eq!(p.source.as_deref(), Some("entrypoint.cpp:3340"));
        assert_eq!(p.message.as_deref(), Some("Starting CCMEXEC service..."));
    }

    #[test]
    fn parses_json_line() {
        let line = r#"{"level":"error","msg":"database error","timestamp":"2026-04-27T10:00:00Z","logger":"db"}"#;
        let p = parse_with_format(line, FileFormat::Json).unwrap();
        assert_eq!(p.level, Some(LogLevel::Error));
        assert_eq!(p.message.as_deref(), Some("database error"));
        assert_eq!(p.component.as_deref(), Some("db"));
    }

    #[test]
    fn parses_nginx_line() {
        let line = r#"192.168.1.1 - - [27/Apr/2026:12:34:56 +0000] "GET /index.html HTTP/1.1" 200 1234 "-" "Mozilla/5.0""#;
        let p = parse_with_format(line, FileFormat::Nginx).unwrap();
        assert_eq!(p.level, Some(LogLevel::Info));
        assert_eq!(p.source.as_deref(), Some("192.168.1.1"));
        assert_eq!(p.message.as_deref(), Some("GET /index.html HTTP/1.1"));
    }

    #[test]
    fn parses_nginx_error() {
        let line = r#"192.168.1.1 - - [27/Apr/2026:12:34:56 +0000] "GET /missing HTTP/1.1" 404 1234 "-" "Mozilla/5.0""#;
        let p = parse_with_format(line, FileFormat::Nginx).unwrap();
        assert_eq!(p.level, Some(LogLevel::Warn));
    }

    #[test]
    fn parses_nginx_server_error() {
        let line = r#"192.168.1.1 - - [27/Apr/2026:12:34:56 +0000] "GET /api HTTP/1.1" 503 1234 "-" "Mozilla/5.0""#;
        let p = parse_with_format(line, FileFormat::Nginx).unwrap();
        assert_eq!(p.level, Some(LogLevel::Error));
    }
}
