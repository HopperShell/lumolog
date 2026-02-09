use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Syslog,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    pub fn short_name(self) -> &'static str {
        match self {
            LogLevel::Trace => "TRC",
            LogLevel::Debug => "DBG",
            LogLevel::Info => "INF",
            LogLevel::Warn => "WRN",
            LogLevel::Error => "ERR",
            LogLevel::Fatal => "FTL",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedLine {
    pub raw: String,
    pub level: Option<LogLevel>,
    pub timestamp: Option<String>,
    pub message: String,
    pub format: LogFormat,
    pub pretty_json: Option<String>,
}

static SYSLOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+(.+)$").unwrap()
});

static PLAIN_TIMESTAMP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}[^\s]*)").unwrap());

static LEVEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(TRACE|DEBUG|INFO|NOTICE|WARN(?:ING)?|ERROR|FATAL|CRITICAL|SEVERE|EMERGENCY|EMERG|ALERT|PANIC)\b").unwrap()
});

pub fn detect_format(lines: &[String]) -> LogFormat {
    let sample: Vec<&str> = lines.iter().take(10).map(|s| s.as_str()).collect();
    if sample.is_empty() {
        return LogFormat::Plain;
    }

    let json_count = sample
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('{') && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        })
        .count();
    if json_count > sample.len() / 2 {
        return LogFormat::Json;
    }

    let syslog_count = sample
        .iter()
        .filter(|line| SYSLOG_RE.is_match(line))
        .count();
    if syslog_count > sample.len() / 2 {
        return LogFormat::Syslog;
    }

    LogFormat::Plain
}

pub fn parse_line(raw: &str, format: LogFormat) -> ParsedLine {
    match format {
        LogFormat::Json => parse_json_line(raw),
        LogFormat::Syslog => parse_syslog_line(raw),
        LogFormat::Plain => parse_plain_line(raw),
    }
}

fn parse_json_line(raw: &str) -> ParsedLine {
    let trimmed = raw.trim();
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => {
            let level = value
                .get("level")
                .or_else(|| value.get("severity"))
                .or_else(|| value.get("log.level"))
                .and_then(|v| {
                    v.as_str()
                        .and_then(parse_level_str)
                        .or_else(|| v.as_u64().and_then(parse_numeric_level))
                });

            let timestamp = value
                .get("timestamp")
                .or_else(|| value.get("time"))
                .or_else(|| value.get("@timestamp"))
                .or_else(|| value.get("ts"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let message = value
                .get("message")
                .or_else(|| value.get("msg"))
                .and_then(|v| v.as_str())
                .unwrap_or(trimmed)
                .to_string();

            let pretty = serde_json::to_string_pretty(&value).ok();

            ParsedLine {
                raw: raw.to_string(),
                level,
                timestamp,
                message,
                format: LogFormat::Json,
                pretty_json: pretty,
            }
        }
        Err(_) => ParsedLine {
            raw: raw.to_string(),
            level: None,
            timestamp: None,
            message: raw.to_string(),
            format: LogFormat::Json,
            pretty_json: None,
        },
    }
}

fn parse_syslog_line(raw: &str) -> ParsedLine {
    let (timestamp, message) = if let Some(caps) = SYSLOG_RE.captures(raw) {
        (Some(caps[1].to_string()), caps[3].to_string())
    } else {
        (None, raw.to_string())
    };

    let level = LEVEL_RE.find(raw).and_then(|m| parse_level_str(m.as_str()));

    ParsedLine {
        raw: raw.to_string(),
        level,
        timestamp,
        message,
        format: LogFormat::Syslog,
        pretty_json: None,
    }
}

fn parse_plain_line(raw: &str) -> ParsedLine {
    let timestamp = PLAIN_TIMESTAMP_RE.find(raw).map(|m| m.as_str().to_string());

    let level = LEVEL_RE.find(raw).and_then(|m| parse_level_str(m.as_str()));

    ParsedLine {
        raw: raw.to_string(),
        level,
        timestamp,
        message: raw.to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
    }
}

fn parse_level_str(s: &str) -> Option<LogLevel> {
    match s.to_uppercase().as_str() {
        "TRACE" => Some(LogLevel::Trace),
        "DEBUG" => Some(LogLevel::Debug),
        "INFO" | "NOTICE" => Some(LogLevel::Info),
        "WARN" | "WARNING" => Some(LogLevel::Warn),
        "ERROR" | "SEVERE" => Some(LogLevel::Error),
        "FATAL" | "CRITICAL" | "EMERGENCY" | "EMERG" | "ALERT" | "PANIC" => Some(LogLevel::Fatal),
        _ => None,
    }
}

/// Parse numeric log levels used by Bunyan, Pino, and similar JSON loggers.
/// Convention: 10=trace, 20=debug, 30=info, 40=warn, 50=error, 60=fatal.
/// Uses ranges to handle custom intermediate levels.
fn parse_numeric_level(n: u64) -> Option<LogLevel> {
    match n {
        1..=10 => Some(LogLevel::Trace),
        11..=20 => Some(LogLevel::Debug),
        21..=30 => Some(LogLevel::Info),
        31..=40 => Some(LogLevel::Warn),
        41..=50 => Some(LogLevel::Error),
        51..=60 => Some(LogLevel::Fatal),
        _ => None,
    }
}
