use lumolog::parser::{LogFormat, LogLevel, detect_format, parse_line};

// ---------------------------------------------------------------------------
// LogLevel ordering tests
// ---------------------------------------------------------------------------

#[test]
fn test_log_level_ordering() {
    assert!(LogLevel::Trace < LogLevel::Debug);
    assert!(LogLevel::Debug < LogLevel::Info);
    assert!(LogLevel::Info < LogLevel::Warn);
    assert!(LogLevel::Warn < LogLevel::Error);
    assert!(LogLevel::Error < LogLevel::Fatal);
}

#[test]
fn test_log_level_sort() {
    let mut levels = vec![
        LogLevel::Error,
        LogLevel::Trace,
        LogLevel::Warn,
        LogLevel::Info,
    ];
    levels.sort();
    assert_eq!(
        levels,
        vec![
            LogLevel::Trace,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error
        ]
    );
}

#[test]
fn test_detect_json_format() {
    let lines = vec![
        r#"{"timestamp":"2024-01-15T08:30:01Z","level":"info","message":"test"}"#.to_string(),
        r#"{"timestamp":"2024-01-15T08:30:02Z","level":"debug","message":"test2"}"#.to_string(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Json);
}

#[test]
fn test_detect_syslog_format() {
    let lines = vec![
        "Jan 15 08:30:01 myhost sshd[1234]: Accepted publickey".to_string(),
        "Jan 15 08:30:02 myhost kernel: something".to_string(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Syslog);
}

#[test]
fn test_detect_plain_format() {
    let lines = vec![
        "2024-01-15 08:30:01 INFO  Application starting up".to_string(),
        "2024-01-15 08:30:02 DEBUG Loading configuration".to_string(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Plain);
}

#[test]
fn test_parse_json_line() {
    let line =
        r#"{"timestamp":"2024-01-15T08:30:05Z","level":"error","message":"Failed to connect"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Error));
    assert!(parsed.timestamp.is_some());
    assert!(parsed.message.contains("Failed to connect"));
}

#[test]
fn test_parse_plain_line_error() {
    let line = "2024-01-15 08:30:05 ERROR Failed to connect to redis";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Error));
}

#[test]
fn test_parse_plain_line_warn() {
    let line = "2024-01-15 08:30:03 WARN  Cache miss rate above threshold";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Warn));
}

#[test]
fn test_parse_syslog_line() {
    let line = "Jan 15 08:30:01 myhost sshd[1234]: Accepted publickey for user";
    let parsed = parse_line(line, LogFormat::Syslog);
    assert!(parsed.timestamp.is_some());
}

// ---------------------------------------------------------------------------
// New level string tests (syslog priorities, Go levels)
// ---------------------------------------------------------------------------

#[test]
fn test_parse_notice_as_info() {
    let line = "2024-01-15 08:30:01 NOTICE User logged in";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Info));
}

#[test]
fn test_parse_emergency_as_fatal() {
    let line = "2024-01-15 08:30:01 EMERGENCY System is unusable";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_parse_emerg_as_fatal() {
    let line = "2024-01-15 08:30:01 EMERG Kernel panic";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_parse_alert_as_fatal() {
    let line = "2024-01-15 08:30:01 ALERT Action must be taken immediately";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_parse_panic_as_fatal() {
    let line = "2024-01-15 08:30:01 PANIC goroutine stack overflow";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_parse_critical_as_fatal() {
    let line = "2024-01-15 08:30:01 CRITICAL Database corruption detected";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_parse_warning_as_warn() {
    // Python/syslog style "WARNING" instead of "WARN"
    let line = "2024-01-15 08:30:01 WARNING Disk space low";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Warn));
}

#[test]
fn test_parse_severe_as_error() {
    // Java's java.util.logging SEVERE level
    let line = "2024-01-15 08:30:01 SEVERE NullPointerException in handler";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Error));
}

// ---------------------------------------------------------------------------
// Numeric JSON level tests (Bunyan/Pino convention)
// ---------------------------------------------------------------------------

#[test]
fn test_parse_json_numeric_level_info() {
    let line = r#"{"level":30,"msg":"Server listening","time":1705302601000}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Info));
}

#[test]
fn test_parse_json_numeric_level_error() {
    let line = r#"{"level":50,"msg":"Connection refused","time":1705302601000}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Error));
}

#[test]
fn test_parse_json_numeric_level_fatal() {
    let line = r#"{"level":60,"msg":"Process crashed","time":1705302601000}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_parse_json_numeric_level_trace() {
    let line = r#"{"level":10,"msg":"Entering function","time":1705302601000}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Trace));
}

#[test]
fn test_parse_json_numeric_level_debug() {
    let line = r#"{"level":20,"msg":"Variable state","time":1705302601000}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Debug));
}

#[test]
fn test_parse_json_numeric_level_warn() {
    let line = r#"{"level":40,"msg":"Deprecated API call","time":1705302601000}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Warn));
}

#[test]
fn test_parse_json_string_level_preferred_over_numeric() {
    // String level should be tried first
    let line = r#"{"level":"error","msg":"Something failed"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Error));
}

#[test]
fn test_parse_json_unknown_numeric_level() {
    let line = r#"{"level":99,"msg":"Unknown level"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, None);
}
