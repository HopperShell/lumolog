use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Syslog,
    Logfmt,
    Klog,
    Log4j,
    PythonLog,
    AccessLog,
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
    pub extra_fields: Vec<(String, String)>,
    pub template: String,
}

static SYSLOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+(.+)$").unwrap()
});

static PLAIN_TIMESTAMP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}[^\s]*)").unwrap());

/// Matches individual key=value tokens for logfmt line detection.
static LOGFMT_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|\s)\w[\w.]*=\S+").unwrap());

static LEVEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(TRACE|DEBUG|INFO|NOTICE|WARN(?:ING)?|ERROR|FATAL|CRITICAL|SEVERE|EMERGENCY|EMERG|ALERT|PANIC)\b").unwrap()
});

/// Klog format: `I0115 08:30:00.000000 12345 file.go:42] actual message`
static KLOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([IWEF])(\d{4}) (\d{2}:\d{2}:\d{2}\.\d+)\s+(\d+) ([^\]]+)\] (.*)$").unwrap()
});

/// Log4j format: `2024-01-15 08:30:00.123 [thread-name] LEVEL com.example.Class - message`
/// Also matches comma decimals (e.g. `08:30:00,123`).
static LOG4J_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}[.,]\d+)\s+\[([^\]]+)\]\s+(\w+)\s+(\S+)\s+-\s+(.*)$").unwrap()
});

/// Python logging format: `2024-01-15 08:30:00,123 - module_name - LEVEL - message`
static PYTHON_LOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2},\d+) - (\S+) - (\w+) - (.*)$").unwrap()
});

/// Apache/Nginx Combined Log Format:
/// `IP - user [timestamp] "METHOD /path HTTP/ver" status bytes "referer" "user-agent"`
static ACCESS_LOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\S+) \S+ (\S+) \[([^\]]+)\] "(\S+) (\S+)[^"]*" (\d{3}) (\d+|-)(?:\s+"([^"]*)" "([^"]*)")?$"#).unwrap()
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

    let logfmt_count = sample
        .iter()
        .filter(|line| LOGFMT_LINE_RE.find_iter(line).count() >= 3)
        .count();
    if logfmt_count > sample.len() / 2 {
        return LogFormat::Logfmt;
    }

    let klog_count = sample.iter().filter(|line| KLOG_RE.is_match(line)).count();
    if klog_count > sample.len() / 2 {
        return LogFormat::Klog;
    }

    let log4j_count = sample.iter().filter(|line| LOG4J_RE.is_match(line)).count();
    if log4j_count > sample.len() / 2 {
        return LogFormat::Log4j;
    }

    let python_count = sample
        .iter()
        .filter(|line| PYTHON_LOG_RE.is_match(line))
        .count();
    if python_count > sample.len() / 2 {
        return LogFormat::PythonLog;
    }

    let access_count = sample
        .iter()
        .filter(|line| ACCESS_LOG_RE.is_match(line))
        .count();
    if access_count > sample.len() / 2 {
        return LogFormat::AccessLog;
    }

    LogFormat::Plain
}

pub fn parse_line(raw: &str, format: LogFormat) -> ParsedLine {
    let mut parsed = match format {
        LogFormat::Json => parse_json_line(raw),
        LogFormat::Syslog => parse_syslog_line(raw),
        LogFormat::Logfmt => parse_logfmt_line(raw),
        LogFormat::Klog => parse_klog_line(raw),
        LogFormat::Log4j => parse_log4j_line(raw),
        LogFormat::PythonLog => parse_python_log_line(raw),
        LogFormat::AccessLog => parse_access_log_line(raw),
        LogFormat::Plain => parse_plain_line(raw),
    };
    parsed.template = compute_template(raw);
    parsed
}

/// Known JSON keys that are already extracted into dedicated ParsedLine fields.
const KNOWN_JSON_KEYS: &[&str] = &[
    "level",
    "severity",
    "log.level",
    "timestamp",
    "time",
    "@timestamp",
    "ts",
    "message",
    "msg",
    "log",
    "stream",
];

/// Matches a single logfmt key=value pair.
/// Captures: (1) key, (2) quoted value without quotes, or (3) unquoted value.
static LOGFMT_PAIR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(\w[\w.]*)=(?:"([^"]*)"|([\S]*))"#).unwrap());

/// Keys that map to the dedicated `level` field.
const LOGFMT_LEVEL_KEYS: &[&str] = &["level", "severity", "log.level"];

/// Keys that map to the dedicated `timestamp` field.
const LOGFMT_TS_KEYS: &[&str] = &["ts", "time", "timestamp", "@timestamp"];

/// Keys that map to the dedicated `message` field.
const LOGFMT_MSG_KEYS: &[&str] = &["msg", "message"];

/// Combined regex matching variable tokens for structural template generation.
/// Replaces URLs, UUIDs, dates, hex addresses, IPs, file paths, and numbers with `*`.
static TEMPLATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        "(?i)",
        r#"https?://[^\s,\]>)"']+"#,
        "|", // URLs
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        "|", // UUIDs
        r"\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}:\d{2}(?:[.,]\d+)?(?:Z|[+-]\d{2}:?\d{2})?)?",
        "|", // Dates
        r"0x[0-9a-f]{4,16}",
        "|", // Hex
        r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}(?::\d{1,5})?",
        "|", // IPv4
        r"(?:\./|~/|/)[\w.\-]+(?:/[\w.\-]+)+",
        "|",                                                      // Paths
        r"\d+(?:\.\d+)?(?:ns|µs|us|ms|s|m|h|d|KB|MB|GB|TB|%|B)?", // Numbers
    ))
    .unwrap()
});

/// Compute a structural template by replacing all variable tokens with `*`.
/// Two lines are "similar" if their templates match exactly.
pub fn compute_template(raw: &str) -> String {
    TEMPLATE_RE.replace_all(raw, "*").to_string()
}

fn format_json_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        // Arrays and objects: compact JSON
        other => serde_json::to_string(other).unwrap_or_default(),
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
                })
                .or_else(|| {
                    // Fallback: scan the message text for a level keyword (e.g. Docker logs
                    // embed the level inside the "log" value, not as a separate key).
                    let msg = value
                        .get("message")
                        .or_else(|| value.get("msg"))
                        .or_else(|| value.get("log"))
                        .and_then(|v| v.as_str())?;
                    LEVEL_RE.find(msg).and_then(|m| parse_level_str(m.as_str()))
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
                .or_else(|| value.get("log"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim_end_matches('\n'))
                .unwrap_or(trimmed)
                .to_string();

            let pretty = serde_json::to_string_pretty(&value).ok();

            // Collect extra fields (keys not in KNOWN_JSON_KEYS).
            // serde_json preserves insertion order with its default Map (backed by BTreeMap
            // when the "preserve_order" feature is off), so keys come out alphabetically.
            let extra_fields = value
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .filter(|(k, _)| !KNOWN_JSON_KEYS.contains(&k.as_str()))
                        .map(|(k, v)| (k.clone(), format_json_value(v)))
                        .collect()
                })
                .unwrap_or_default();

            ParsedLine {
                raw: raw.to_string(),
                level,
                timestamp,
                message,
                format: LogFormat::Json,
                pretty_json: pretty,
                extra_fields,
                template: String::new(),
            }
        }
        Err(_) => ParsedLine {
            raw: raw.to_string(),
            level: None,
            timestamp: None,
            message: raw.to_string(),
            format: LogFormat::Json,
            pretty_json: None,
            extra_fields: Vec::new(),
            template: String::new(),
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
        extra_fields: Vec::new(),
        template: String::new(),
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
        extra_fields: Vec::new(),
        template: String::new(),
    }
}

fn parse_logfmt_line(raw: &str) -> ParsedLine {
    let mut level = None;
    let mut timestamp = None;
    let mut message = None;
    let mut extra_fields = Vec::new();

    for caps in LOGFMT_PAIR_RE.captures_iter(raw) {
        let key = &caps[1];
        let value = caps
            .get(2)
            .or_else(|| caps.get(3))
            .map(|m| m.as_str())
            .unwrap_or("");

        if LOGFMT_LEVEL_KEYS.contains(&key) && level.is_none() {
            level = parse_level_str(value);
        } else if LOGFMT_TS_KEYS.contains(&key) && timestamp.is_none() {
            timestamp = Some(value.to_string());
        } else if LOGFMT_MSG_KEYS.contains(&key) && message.is_none() {
            message = Some(value.to_string());
        } else {
            extra_fields.push((key.to_string(), value.to_string()));
        }
    }

    ParsedLine {
        raw: raw.to_string(),
        level,
        timestamp,
        message: message.unwrap_or_else(|| raw.to_string()),
        format: LogFormat::Logfmt,
        pretty_json: None,
        extra_fields,
        template: String::new(),
    }
}

fn parse_klog_line(raw: &str) -> ParsedLine {
    if let Some(caps) = KLOG_RE.captures(raw) {
        let level = match &caps[1] {
            "I" => Some(LogLevel::Info),
            "W" => Some(LogLevel::Warn),
            "E" => Some(LogLevel::Error),
            "F" => Some(LogLevel::Fatal),
            _ => None,
        };
        // Timestamp: MMDD HH:MM:SS.micros
        let timestamp = Some(format!("{} {}", &caps[2], &caps[3]));
        let message = caps[6].to_string();
        let extra_fields = vec![
            ("pid".to_string(), caps[4].to_string()),
            ("source".to_string(), caps[5].to_string()),
        ];

        ParsedLine {
            raw: raw.to_string(),
            level,
            timestamp,
            message,
            format: LogFormat::Klog,
            pretty_json: None,
            extra_fields,
            template: String::new(),
        }
    } else {
        ParsedLine {
            raw: raw.to_string(),
            level: None,
            timestamp: None,
            message: raw.to_string(),
            format: LogFormat::Klog,
            pretty_json: None,
            extra_fields: Vec::new(),
            template: String::new(),
        }
    }
}

fn parse_log4j_line(raw: &str) -> ParsedLine {
    if let Some(caps) = LOG4J_RE.captures(raw) {
        let timestamp = Some(caps[1].to_string());
        let thread = caps[2].to_string();
        let level = parse_level_str(&caps[3]);
        let class = caps[4].to_string();
        let message = caps[5].to_string();
        let extra_fields = vec![("thread".to_string(), thread), ("class".to_string(), class)];

        ParsedLine {
            raw: raw.to_string(),
            level,
            timestamp,
            message,
            format: LogFormat::Log4j,
            pretty_json: None,
            extra_fields,
            template: String::new(),
        }
    } else {
        ParsedLine {
            raw: raw.to_string(),
            level: LEVEL_RE.find(raw).and_then(|m| parse_level_str(m.as_str())),
            timestamp: PLAIN_TIMESTAMP_RE.find(raw).map(|m| m.as_str().to_string()),
            message: raw.to_string(),
            format: LogFormat::Log4j,
            pretty_json: None,
            extra_fields: Vec::new(),
            template: String::new(),
        }
    }
}

fn parse_python_log_line(raw: &str) -> ParsedLine {
    if let Some(caps) = PYTHON_LOG_RE.captures(raw) {
        let timestamp = Some(caps[1].to_string());
        let module = caps[2].to_string();
        let level = parse_level_str(&caps[3]);
        let message = caps[4].to_string();
        let extra_fields = vec![("module".to_string(), module)];

        ParsedLine {
            raw: raw.to_string(),
            level,
            timestamp,
            message,
            format: LogFormat::PythonLog,
            pretty_json: None,
            extra_fields,
            template: String::new(),
        }
    } else {
        ParsedLine {
            raw: raw.to_string(),
            level: LEVEL_RE.find(raw).and_then(|m| parse_level_str(m.as_str())),
            timestamp: PLAIN_TIMESTAMP_RE.find(raw).map(|m| m.as_str().to_string()),
            message: raw.to_string(),
            format: LogFormat::PythonLog,
            pretty_json: None,
            extra_fields: Vec::new(),
            template: String::new(),
        }
    }
}

/// Map HTTP status code to log level.
fn status_to_level(status: u16) -> Option<LogLevel> {
    match status {
        500..=599 => Some(LogLevel::Error),
        400..=499 => Some(LogLevel::Warn),
        _ => Some(LogLevel::Info),
    }
}

fn parse_access_log_line(raw: &str) -> ParsedLine {
    if let Some(caps) = ACCESS_LOG_RE.captures(raw) {
        let ip = caps[1].to_string();
        let user = caps[2].to_string();
        let timestamp = Some(caps[3].to_string());
        let method = caps[4].to_string();
        let path = caps[5].to_string();
        let status: u16 = caps[6].parse().unwrap_or(0);
        let bytes = caps[7].to_string();
        let level = status_to_level(status);
        let message = format!("{} {} {}", method, path, status);

        let mut extra_fields = Vec::new();
        extra_fields.push(("ip".to_string(), ip));
        if user != "-" {
            extra_fields.push(("user".to_string(), user));
        }
        extra_fields.push(("bytes".to_string(), bytes));
        if let Some(referer) = caps.get(8) {
            let r = referer.as_str();
            if r != "-" {
                extra_fields.push(("referer".to_string(), r.to_string()));
            }
        }
        if let Some(ua) = caps.get(9) {
            extra_fields.push(("ua".to_string(), ua.as_str().to_string()));
        }

        ParsedLine {
            raw: raw.to_string(),
            level,
            timestamp,
            message,
            format: LogFormat::AccessLog,
            pretty_json: None,
            extra_fields,
            template: String::new(),
        }
    } else {
        ParsedLine {
            raw: raw.to_string(),
            level: None,
            timestamp: None,
            message: raw.to_string(),
            format: LogFormat::AccessLog,
            pretty_json: None,
            extra_fields: Vec::new(),
            template: String::new(),
        }
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
