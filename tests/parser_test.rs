use lumolog::parser::{detect_format, LogFormat, parse_line, LogLevel};

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
    let line = r#"{"timestamp":"2024-01-15T08:30:05Z","level":"error","message":"Failed to connect"}"#;
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
