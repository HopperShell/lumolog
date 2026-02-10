use lumolog::parser::{LogFormat, LogLevel, compute_template, detect_format, parse_line};

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

// ---------------------------------------------------------------------------
// Extra fields tests
// ---------------------------------------------------------------------------

#[test]
fn test_parse_json_extra_fields_captured() {
    let line = r#"{"level":"error","message":"Failed to connect","error":"Connection refused","host":"localhost:6379"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.extra_fields.len(), 2);
    assert!(parsed
        .extra_fields
        .iter()
        .any(|(k, v)| k == "error" && v == r#""Connection refused""#));
    assert!(parsed
        .extra_fields
        .iter()
        .any(|(k, v)| k == "host" && v == r#""localhost:6379""#));
}

#[test]
fn test_parse_json_extra_fields_numeric_values_bare() {
    let line = r#"{"level":"info","message":"Request handled","duration_ms":23,"status":200}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert!(parsed
        .extra_fields
        .iter()
        .any(|(k, v)| k == "duration_ms" && v == "23"));
    assert!(parsed
        .extra_fields
        .iter()
        .any(|(k, v)| k == "status" && v == "200"));
}

#[test]
fn test_parse_json_no_extra_fields_when_only_known_keys() {
    let line =
        r#"{"level":"info","timestamp":"2024-01-15T08:30:05Z","message":"All fields known"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert!(parsed.extra_fields.is_empty());
}

#[test]
fn test_parse_json_all_known_keys_excluded() {
    // Every known key variant should be excluded from extra_fields
    let line = r#"{"level":"info","severity":"info","log.level":"info","timestamp":"t","time":"t","@timestamp":"t","ts":"t","message":"m","msg":"m","extra":"yes"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.extra_fields.len(), 1);
    assert_eq!(parsed.extra_fields[0].0, "extra");
}

// ---------------------------------------------------------------------------
// Template computation tests
// ---------------------------------------------------------------------------

#[test]
fn test_template_replaces_ip_and_number() {
    let t = compute_template("Failed to connect to 10.0.0.5:3306 after 3 retries");
    assert_eq!(t, "Failed to connect to * after * retries");
}

#[test]
fn test_template_replaces_uuid() {
    let t = compute_template("Request f47ac10b-58cc-4372-a567-0e02b2c3d479 processed");
    assert_eq!(t, "Request * processed");
}

#[test]
fn test_template_replaces_url() {
    let t = compute_template("Fetching https://api.example.com/data?id=42 done");
    assert_eq!(t, "Fetching * done");
}

#[test]
fn test_template_replaces_timestamp() {
    let t = compute_template("Event at 2024-01-15T08:30:00Z completed");
    assert_eq!(t, "Event at * completed");
}

#[test]
fn test_template_replaces_hex() {
    let t = compute_template("Segfault at 0x7fff5fbff8c0 in thread 3");
    assert_eq!(t, "Segfault at * in thread *");
}

#[test]
fn test_template_replaces_path() {
    let t = compute_template("Loading config from /etc/app/config.yaml");
    assert_eq!(t, "Loading config from *");
}

#[test]
fn test_template_similar_lines_match() {
    let a = compute_template("Failed to connect to 10.0.0.5:3306 after 3 retries");
    let b = compute_template("Failed to connect to 192.168.1.1:5432 after 10 retries");
    assert_eq!(a, b);
}

#[test]
fn test_template_different_structure_no_match() {
    let a = compute_template("Failed to connect to 10.0.0.5:3306 after 3 retries");
    let b = compute_template("User 42 logged in from 10.0.0.5");
    assert_ne!(a, b);
}

#[test]
fn test_parse_line_sets_template() {
    let parsed = parse_line("2024-01-15 ERROR Connection to 10.0.0.1 refused", LogFormat::Plain);
    assert!(!parsed.template.is_empty());
    assert!(parsed.template.contains("*"));
}

#[test]
fn test_detect_logfmt_format() {
    let lines: Vec<String> = vec![
        "level=info ts=2024-01-15T08:30:01Z msg=\"server starting\" addr=0.0.0.0:8080".into(),
        "level=debug ts=2024-01-15T08:30:02Z msg=\"connected to database\" host=localhost:5432".into(),
        "level=warn ts=2024-01-15T08:30:03Z msg=\"cache miss\" rate=0.45".into(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Logfmt);
}

#[test]
fn test_detect_logfmt_without_quotes() {
    let lines: Vec<String> = vec![
        "level=info ts=2024-01-15T08:30:01Z msg=starting addr=0.0.0.0:8080".into(),
        "level=debug ts=2024-01-15T08:30:02Z msg=connected host=localhost".into(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Logfmt);
}

#[test]
fn test_plain_kv_not_detected_as_logfmt() {
    // Only 1 key=value pair per line — not logfmt
    let lines: Vec<String> = vec![
        "2024-01-15 ERROR status=500".into(),
        "2024-01-15 INFO Starting up".into(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Plain);
}

// ---------------------------------------------------------------------------
// Logfmt parsing tests
// ---------------------------------------------------------------------------

#[test]
fn test_parse_logfmt_extracts_level() {
    let line = r#"level=error ts=2024-01-15T08:30:05Z msg="connection refused" host=localhost:6379"#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.level, Some(LogLevel::Error));
    assert_eq!(parsed.format, LogFormat::Logfmt);
}

#[test]
fn test_parse_logfmt_extracts_timestamp() {
    let line = r#"level=info ts=2024-01-15T08:30:01Z msg="server starting""#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.timestamp, Some("2024-01-15T08:30:01Z".to_string()));
}

#[test]
fn test_parse_logfmt_extracts_message() {
    let line = r#"level=info ts=2024-01-15T08:30:01Z msg="server starting" addr=0.0.0.0:8080"#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.message, "server starting");
}

#[test]
fn test_parse_logfmt_msg_key() {
    let line = r#"level=info msg="hello world""#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.message, "hello world");
}

#[test]
fn test_parse_logfmt_message_key() {
    let line = r#"level=info message="hello world""#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.message, "hello world");
}

#[test]
fn test_parse_logfmt_extra_fields() {
    let line = r#"level=info ts=2024-01-15T08:30:01Z msg="request handled" method=GET status=200 duration=23ms"#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.extra_fields.len(), 3);
    assert!(parsed.extra_fields.iter().any(|(k, v)| k == "method" && v == "GET"));
    assert!(parsed.extra_fields.iter().any(|(k, v)| k == "status" && v == "200"));
    assert!(parsed.extra_fields.iter().any(|(k, v)| k == "duration" && v == "23ms"));
}

#[test]
fn test_parse_logfmt_unquoted_msg() {
    let line = "level=info msg=starting addr=0.0.0.0:8080";
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.message, "starting");
}

#[test]
fn test_parse_logfmt_severity_key() {
    let line = r#"severity=warn msg="disk usage high""#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.level, Some(LogLevel::Warn));
}

#[test]
fn test_parse_logfmt_time_key() {
    let line = r#"level=info time=2024-01-15T08:30:01Z msg="hello""#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.timestamp, Some("2024-01-15T08:30:01Z".to_string()));
}

#[test]
fn test_parse_logfmt_quoted_value_with_spaces() {
    let line = r#"level=error msg="connection refused" err="dial tcp 127.0.0.1:6379: connect: connection refused""#;
    let parsed = parse_line(line, LogFormat::Logfmt);
    assert_eq!(parsed.message, "connection refused");
    assert!(parsed.extra_fields.iter().any(|(k, v)| k == "err" && v.contains("dial tcp")));
}

#[test]
fn test_detect_logfmt_from_sample_file() {
    let content = std::fs::read_to_string("testdata/sample_logfmt.log").unwrap();
    let lines: Vec<String> = content.lines().filter(|l| !l.is_empty()).map(String::from).collect();
    assert_eq!(detect_format(&lines), LogFormat::Logfmt);

    let parsed = parse_line(&lines[0], LogFormat::Logfmt);
    assert_eq!(parsed.level, Some(LogLevel::Info));
    assert_eq!(parsed.timestamp, Some("2024-01-15T08:30:01Z".to_string()));
    assert_eq!(parsed.message, "server starting");
    assert!(parsed.extra_fields.iter().any(|(k, _)| k == "caller"));
    assert!(parsed.extra_fields.iter().any(|(k, _)| k == "addr"));
}
