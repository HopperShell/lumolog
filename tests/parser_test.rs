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
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "error" && v == r#""Connection refused""#)
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "host" && v == r#""localhost:6379""#)
    );
}

#[test]
fn test_parse_json_extra_fields_numeric_values_bare() {
    let line = r#"{"level":"info","message":"Request handled","duration_ms":23,"status":200}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "duration_ms" && v == "23")
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "status" && v == "200")
    );
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
    let parsed = parse_line(
        "2024-01-15 ERROR Connection to 10.0.0.1 refused",
        LogFormat::Plain,
    );
    assert!(!parsed.template.is_empty());
    assert!(parsed.template.contains("*"));
}

#[test]
fn test_detect_logfmt_format() {
    let lines: Vec<String> = vec![
        "level=info ts=2024-01-15T08:30:01Z msg=\"server starting\" addr=0.0.0.0:8080".into(),
        "level=debug ts=2024-01-15T08:30:02Z msg=\"connected to database\" host=localhost:5432"
            .into(),
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
    let line =
        r#"level=error ts=2024-01-15T08:30:05Z msg="connection refused" host=localhost:6379"#;
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
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "method" && v == "GET")
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "status" && v == "200")
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "duration" && v == "23ms")
    );
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
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "err" && v.contains("dial tcp"))
    );
}

#[test]
fn test_detect_logfmt_from_sample_file() {
    let content = std::fs::read_to_string("testdata/sample_logfmt.log").unwrap();
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    assert_eq!(detect_format(&lines), LogFormat::Logfmt);

    let parsed = parse_line(&lines[0], LogFormat::Logfmt);
    assert_eq!(parsed.level, Some(LogLevel::Info));
    assert_eq!(parsed.timestamp, Some("2024-01-15T08:30:01Z".to_string()));
    assert_eq!(parsed.message, "server starting");
    assert!(parsed.extra_fields.iter().any(|(k, _)| k == "caller"));
    assert!(parsed.extra_fields.iter().any(|(k, _)| k == "addr"));
}

// ---------------------------------------------------------------------------
// Docker JSON log wrapper tests
// ---------------------------------------------------------------------------

#[test]
fn test_docker_json_log_extracts_message() {
    let line = r#"{"log":"Server starting on port 3000\n","stream":"stdout","time":"2024-01-15T08:30:01.000000000Z"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.message, "Server starting on port 3000");
}

#[test]
fn test_docker_json_log_extracts_timestamp() {
    let line = r#"{"log":"Hello\n","stream":"stdout","time":"2024-01-15T08:30:01.000000000Z"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(
        parsed.timestamp,
        Some("2024-01-15T08:30:01.000000000Z".to_string())
    );
}

#[test]
fn test_docker_json_log_known_keys_suppressed() {
    let line = r#"{"log":"Hello\n","stream":"stdout","time":"2024-01-15T08:30:01.000000000Z"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    // log, stream, and time are all in KNOWN_JSON_KEYS — no extra fields
    assert!(parsed.extra_fields.is_empty());
}

#[test]
fn test_docker_json_log_strips_trailing_newline() {
    let line = r#"{"log":"message with trailing newline\n","stream":"stdout","time":"2024-01-15T08:30:01Z"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert!(!parsed.message.ends_with('\n'));
    assert_eq!(parsed.message, "message with trailing newline");
}

#[test]
fn test_docker_json_level_fallback_from_message() {
    // Docker logs have no "level" key — level is embedded in the log text
    let line = r#"{"log":"2024-01-15T08:30:01Z ERROR Connection refused\n","stream":"stderr","time":"2024-01-15T08:30:01.000000000Z"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Error));
}

#[test]
fn test_docker_json_no_level_when_absent() {
    // Plain message with no level keyword → level should be None
    let line = r#"{"log":"Starting myapp v2.4.1\n","stream":"stdout","time":"2024-01-15T08:30:00.000000000Z"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, None);
}

#[test]
fn test_docker_json_sample_file() {
    let content = std::fs::read_to_string("testdata/sample_docker.log").unwrap();
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    assert_eq!(detect_format(&lines), LogFormat::Json);
    assert_eq!(lines.len(), 10);

    // Line 0: plain startup banner, no level
    let p0 = parse_line(&lines[0], LogFormat::Json);
    assert_eq!(p0.message, "Starting myapp v2.4.1");
    assert_eq!(p0.level, None);
    assert!(p0.timestamp.is_some());

    // Line 1: has INFO in the message text
    let p1 = parse_line(&lines[1], LogFormat::Json);
    assert_eq!(p1.level, Some(LogLevel::Info));

    // Line 5: ERROR on stderr
    let p5 = parse_line(&lines[5], LogFormat::Json);
    assert_eq!(p5.level, Some(LogLevel::Error));

    // Line 6: "panic:" on stderr — PANIC matches as Fatal
    let p6 = parse_line(&lines[6], LogFormat::Json);
    assert_eq!(
        p6.message,
        "panic: runtime error: index out of range [3] with length 2"
    );
    assert_eq!(p6.level, Some(LogLevel::Fatal));
}

// ---------------------------------------------------------------------------
// Klog (Kubernetes) parser tests
// ---------------------------------------------------------------------------

#[test]
fn test_detect_klog_format() {
    let lines: Vec<String> = vec![
        "I0115 08:30:00.000000       1 server.go:42] Starting server on :8080".into(),
        "W0115 08:30:02.234567       1 config.go:18] Deprecated flag used".into(),
        "E0115 08:30:04.456789    1234 handler.go:77] Failed to process request".into(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Klog);
}

#[test]
fn test_parse_klog_info() {
    let line = "I0115 08:30:00.000000       1 server.go:42] Starting server on :8080";
    let parsed = parse_line(line, LogFormat::Klog);
    assert_eq!(parsed.level, Some(LogLevel::Info));
    assert_eq!(parsed.timestamp, Some("0115 08:30:00.000000".to_string()));
    assert_eq!(parsed.message, "Starting server on :8080");
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "pid" && v == "1")
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "source" && v == "server.go:42")
    );
}

#[test]
fn test_parse_klog_warn() {
    let line = "W0115 08:30:02.234567       1 config.go:18] Deprecated flag --insecure-port used";
    let parsed = parse_line(line, LogFormat::Klog);
    assert_eq!(parsed.level, Some(LogLevel::Warn));
}

#[test]
fn test_parse_klog_error() {
    let line = "E0115 08:30:04.456789    1234 handler.go:77] Failed to process request: context deadline exceeded";
    let parsed = parse_line(line, LogFormat::Klog);
    assert_eq!(parsed.level, Some(LogLevel::Error));
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "pid" && v == "1234")
    );
}

#[test]
fn test_parse_klog_fatal() {
    let line = "F0115 08:30:08.890123       1 server.go:99] Unable to bind to port 6443: address already in use";
    let parsed = parse_line(line, LogFormat::Klog);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_klog_sample_file() {
    let content = std::fs::read_to_string("testdata/sample_klog.log").unwrap();
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    assert_eq!(detect_format(&lines), LogFormat::Klog);
    assert_eq!(lines.len(), 10);

    let p0 = parse_line(&lines[0], LogFormat::Klog);
    assert_eq!(p0.level, Some(LogLevel::Info));
    assert_eq!(p0.message, "Starting server on :8080");

    let p8 = parse_line(&lines[8], LogFormat::Klog);
    assert_eq!(p8.level, Some(LogLevel::Fatal));
}

// ---------------------------------------------------------------------------
// Log4j / Java logging parser tests
// ---------------------------------------------------------------------------

#[test]
fn test_detect_log4j_format() {
    let lines: Vec<String> = vec![
        "2024-01-15 08:30:00.123 [main] INFO  com.example.Application - Starting up".into(),
        "2024-01-15 08:30:01.234 [main] DEBUG com.example.db.Pool - Init pool".into(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Log4j);
}

#[test]
fn test_parse_log4j_info() {
    let line =
        "2024-01-15 08:30:00.123 [main] INFO  com.example.Application - Application starting up";
    let parsed = parse_line(line, LogFormat::Log4j);
    assert_eq!(parsed.level, Some(LogLevel::Info));
    assert_eq!(
        parsed.timestamp,
        Some("2024-01-15 08:30:00.123".to_string())
    );
    assert_eq!(parsed.message, "Application starting up");
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "thread" && v == "main")
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "class" && v == "com.example.Application")
    );
}

#[test]
fn test_parse_log4j_error() {
    let line = "2024-01-15 08:30:05.678 [http-nio-8080-exec-1] ERROR com.example.service.UserService - Failed to fetch user 42: Connection refused";
    let parsed = parse_line(line, LogFormat::Log4j);
    assert_eq!(parsed.level, Some(LogLevel::Error));
    assert_eq!(
        parsed.message,
        "Failed to fetch user 42: Connection refused"
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "thread" && v == "http-nio-8080-exec-1")
    );
}

#[test]
fn test_parse_log4j_fatal() {
    let line = "2024-01-15 08:30:09.012 [main] FATAL com.example.Application - Unrecoverable error: out of memory";
    let parsed = parse_line(line, LogFormat::Log4j);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_log4j_sample_file() {
    let content = std::fs::read_to_string("testdata/sample_log4j.log").unwrap();
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    assert_eq!(detect_format(&lines), LogFormat::Log4j);
    assert_eq!(lines.len(), 10);

    let p0 = parse_line(&lines[0], LogFormat::Log4j);
    assert_eq!(p0.level, Some(LogLevel::Info));
    assert_eq!(p0.message, "Application starting up");

    let p5 = parse_line(&lines[5], LogFormat::Log4j);
    assert_eq!(p5.level, Some(LogLevel::Error));

    let p9 = parse_line(&lines[9], LogFormat::Log4j);
    assert_eq!(p9.level, Some(LogLevel::Fatal));
}

// ---------------------------------------------------------------------------
// Python logging parser tests
// ---------------------------------------------------------------------------

#[test]
fn test_detect_python_log_format() {
    let lines: Vec<String> = vec![
        "2024-01-15 08:30:00,123 - myapp - INFO - Application started".into(),
        "2024-01-15 08:30:01,234 - myapp.config - DEBUG - Loaded config".into(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::PythonLog);
}

#[test]
fn test_parse_python_log_info() {
    let line = "2024-01-15 08:30:00,123 - myapp - INFO - Application started successfully";
    let parsed = parse_line(line, LogFormat::PythonLog);
    assert_eq!(parsed.level, Some(LogLevel::Info));
    assert_eq!(
        parsed.timestamp,
        Some("2024-01-15 08:30:00,123".to_string())
    );
    assert_eq!(parsed.message, "Application started successfully");
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "module" && v == "myapp")
    );
}

#[test]
fn test_parse_python_log_warning() {
    let line = "2024-01-15 08:30:03,456 - myapp.web - WARNING - Slow query detected: 2.3s";
    let parsed = parse_line(line, LogFormat::PythonLog);
    assert_eq!(parsed.level, Some(LogLevel::Warn));
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "module" && v == "myapp.web")
    );
}

#[test]
fn test_parse_python_log_critical() {
    let line = "2024-01-15 08:30:09,012 - myapp - CRITICAL - Unhandled exception in main loop";
    let parsed = parse_line(line, LogFormat::PythonLog);
    assert_eq!(parsed.level, Some(LogLevel::Fatal));
}

#[test]
fn test_python_log_sample_file() {
    let content = std::fs::read_to_string("testdata/sample_python.log").unwrap();
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    assert_eq!(detect_format(&lines), LogFormat::PythonLog);
    assert_eq!(lines.len(), 10);

    let p0 = parse_line(&lines[0], LogFormat::PythonLog);
    assert_eq!(p0.level, Some(LogLevel::Info));
    assert_eq!(p0.message, "Application started successfully");
    assert!(
        p0.extra_fields
            .iter()
            .any(|(k, v)| k == "module" && v == "myapp")
    );

    let p4 = parse_line(&lines[4], LogFormat::PythonLog);
    assert_eq!(p4.level, Some(LogLevel::Error));

    let p9 = parse_line(&lines[9], LogFormat::PythonLog);
    assert_eq!(p9.level, Some(LogLevel::Fatal));
}

// ---------------------------------------------------------------------------
// Apache/Nginx access log parser tests
// ---------------------------------------------------------------------------

#[test]
fn test_detect_access_log_format() {
    let lines: Vec<String> = vec![
        r#"192.168.1.100 - frank [10/Oct/2024:13:55:36 -0700] "GET /api/users HTTP/1.1" 200 2326"#
            .into(),
        r#"10.0.0.1 - - [10/Oct/2024:13:55:37 -0700] "POST /api/login HTTP/1.1" 401 512"#.into(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::AccessLog);
}

#[test]
fn test_parse_access_log_200() {
    let line =
        r#"192.168.1.100 - frank [10/Oct/2024:13:55:36 -0700] "GET /api/users HTTP/1.1" 200 2326"#;
    let parsed = parse_line(line, LogFormat::AccessLog);
    assert_eq!(parsed.level, Some(LogLevel::Info));
    assert_eq!(
        parsed.timestamp,
        Some("10/Oct/2024:13:55:36 -0700".to_string())
    );
    assert_eq!(parsed.message, "GET /api/users 200");
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "ip" && v == "192.168.1.100")
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "user" && v == "frank")
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "bytes" && v == "2326")
    );
}

#[test]
fn test_parse_access_log_401_is_warn() {
    let line = r#"10.0.0.1 - - [10/Oct/2024:13:55:37 -0700] "POST /api/login HTTP/1.1" 401 512"#;
    let parsed = parse_line(line, LogFormat::AccessLog);
    assert_eq!(parsed.level, Some(LogLevel::Warn));
    // user "-" should be excluded from extra_fields
    assert!(!parsed.extra_fields.iter().any(|(k, _)| k == "user"));
}

#[test]
fn test_parse_access_log_500_is_error() {
    let line = r#"10.0.0.5 - - [10/Oct/2024:13:55:40 -0700] "PUT /api/config HTTP/1.1" 500 1024"#;
    let parsed = parse_line(line, LogFormat::AccessLog);
    assert_eq!(parsed.level, Some(LogLevel::Error));
    assert_eq!(parsed.message, "PUT /api/config 500");
}

#[test]
fn test_parse_access_log_combined_format() {
    let line = r#"192.168.1.100 - frank [10/Oct/2024:13:55:41 -0700] "GET /index.html HTTP/1.1" 200 5120 "https://example.com/" "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)""#;
    let parsed = parse_line(line, LogFormat::AccessLog);
    assert_eq!(parsed.level, Some(LogLevel::Info));
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "referer" && v == "https://example.com/")
    );
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "ua" && v.contains("Mozilla"))
    );
}

#[test]
fn test_parse_access_log_combined_dash_referer() {
    let line = r#"10.0.0.1 - - [10/Oct/2024:13:55:42 -0700] "GET /api/health HTTP/1.1" 200 13 "-" "curl/8.1.2""#;
    let parsed = parse_line(line, LogFormat::AccessLog);
    // referer "-" should be excluded
    assert!(!parsed.extra_fields.iter().any(|(k, _)| k == "referer"));
    assert!(
        parsed
            .extra_fields
            .iter()
            .any(|(k, v)| k == "ua" && v == "curl/8.1.2")
    );
}

#[test]
fn test_access_log_sample_file() {
    let content = std::fs::read_to_string("testdata/sample_apache.log").unwrap();
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    assert_eq!(detect_format(&lines), LogFormat::AccessLog);
    assert_eq!(lines.len(), 8);

    let p0 = parse_line(&lines[0], LogFormat::AccessLog);
    assert_eq!(p0.level, Some(LogLevel::Info));
    assert_eq!(p0.message, "GET /api/users 200");

    let p4 = parse_line(&lines[4], LogFormat::AccessLog);
    assert_eq!(p4.level, Some(LogLevel::Error));

    // Combined format line with referer and user-agent
    let p5 = parse_line(&lines[5], LogFormat::AccessLog);
    assert!(p5.extra_fields.iter().any(|(k, _)| k == "referer"));
    assert!(p5.extra_fields.iter().any(|(k, _)| k == "ua"));
}
