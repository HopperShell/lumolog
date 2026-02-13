use lumolog::highlighter::{apply_search_highlight, highlight_line};
use lumolog::parser::{LogFormat, LogLevel, ParsedLine};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

#[test]
fn test_error_line_has_red() {
    let parsed = ParsedLine {
        raw: "2024-01-15 ERROR something broke".to_string(),
        level: Some(LogLevel::Error),
        timestamp: Some("2024-01-15".to_string()),
        message: "something broke".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_red = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Red));
    assert!(has_red, "Error lines should contain red spans");
}

#[test]
fn test_warn_line_has_yellow() {
    let parsed = ParsedLine {
        raw: "2024-01-15 WARN something iffy".to_string(),
        level: Some(LogLevel::Warn),
        timestamp: Some("2024-01-15".to_string()),
        message: "something iffy".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_yellow = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Yellow));
    assert!(has_yellow, "Warn lines should contain yellow spans");
}

#[test]
fn test_info_line_is_dimmed() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO all good".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "all good".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_red = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Red));
    let has_yellow = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Yellow));
    assert!(
        !has_red && !has_yellow,
        "Info lines should not be red or yellow"
    );
}

// ---------------------------------------------------------------------------
// Inline pattern highlighting tests
// ---------------------------------------------------------------------------

#[test]
fn test_ip_address_highlighted() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO Connected from 192.168.1.100 port 52413".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "Connected from 192.168.1.100 port 52413".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_cyan = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Cyan));
    assert!(has_cyan, "IP addresses should be highlighted in cyan");
}

#[test]
fn test_invalid_ip_not_highlighted_as_single_span() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO Version 999.999.999.999 released".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "Version 999.999.999.999 released".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    // 999 > 255, so this should NOT be highlighted as a single IP span.
    // Individual "999.999" segments may match as numbers (that's fine).
    let has_full_ip = styled
        .spans
        .iter()
        .any(|span| span.content.as_ref() == "999.999.999.999");
    assert!(
        !has_full_ip,
        "Invalid IP (octets >255) should not be highlighted as a single IP span"
    );
}

#[test]
fn test_url_highlighted() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO Fetching https://api.example.com/data".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "Fetching https://api.example.com/data".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_blue_underline = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Blue) && span.content.contains("https://"));
    assert!(has_blue_underline, "URLs should be highlighted in blue");
}

#[test]
fn test_uuid_highlighted() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO Request f47ac10b-58cc-4372-a567-0e02b2c3d479 processed".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "Request f47ac10b-58cc-4372-a567-0e02b2c3d479 processed".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_magenta = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Magenta) && span.content.contains("f47ac10b"));
    assert!(has_magenta, "UUIDs should be highlighted in magenta");
}

#[test]
fn test_file_path_highlighted() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO Loading config from /etc/app/config.yaml".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "Loading config from /etc/app/config.yaml".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_cyan_path = styled.spans.iter().any(|span| {
        span.style.fg == Some(Color::Indexed(108)) && span.content.contains("/etc/app")
    });
    assert!(has_cyan_path, "File paths should be highlighted in cyan");
}

#[test]
fn test_http_method_highlighted() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO Request: GET /api/users".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "Request: GET /api/users".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_method = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Magenta) && span.content.as_ref() == "GET");
    assert!(
        has_method,
        "HTTP methods should be highlighted in magenta+bold"
    );
}

#[test]
fn test_quoted_string_highlighted() {
    let parsed = ParsedLine {
        raw: r#"2024-01-15 ERROR Cannot open file "config.yaml": permission denied"#.to_string(),
        level: Some(LogLevel::Error),
        timestamp: Some("2024-01-15".to_string()),
        message: r#"Cannot open file "config.yaml": permission denied"#.to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_quoted = styled.spans.iter().any(|span| {
        span.style.fg == Some(Color::Indexed(222)) && span.content.contains("config.yaml")
    });
    assert!(has_quoted, "Quoted strings should be highlighted in gold");
}

#[test]
fn test_key_value_highlighted() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO host=localhost:6379 status=connected".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "host=localhost:6379 status=connected".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_kv = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Blue) && span.content.contains("host="));
    assert!(has_kv, "Key=value keys should be highlighted in blue+bold");
}

#[test]
fn test_syslog_ip_highlighted() {
    let parsed = ParsedLine {
        raw: "Jan 15 08:30:01 myhost sshd[1234]: Accepted publickey from 192.168.1.100 port 52413"
            .to_string(),
        level: None,
        timestamp: Some("Jan 15 08:30:01".to_string()),
        message: "sshd[1234]: Accepted publickey from 192.168.1.100 port 52413".to_string(),
        format: LogFormat::Syslog,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_cyan = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Cyan) && span.content.contains("192.168"));
    assert!(has_cyan, "IPs in syslog lines should be highlighted");
}

#[test]
fn test_json_message_patterns_highlighted() {
    let parsed = ParsedLine {
        raw: r#"{"level":"info","message":"Connected from 10.0.0.1"}"#.to_string(),
        level: Some(LogLevel::Info),
        timestamp: None,
        message: "Connected from 10.0.0.1".to_string(),
        format: LogFormat::Json,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_cyan = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Cyan) && span.content.contains("10.0.0.1"));
    assert!(has_cyan, "IPs in JSON messages should be highlighted");
}

#[test]
fn test_plain_line_without_patterns() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO Application starting up".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "Application starting up".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    // Should still work fine - timestamp in gray, rest in green (info)
    let has_gray = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::DarkGray));
    let has_green = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Green));
    assert!(has_gray, "Timestamp should be gray");
    assert!(has_green, "Info text should be green");
}

#[test]
fn test_url_takes_priority_over_path() {
    let parsed = ParsedLine {
        raw: "2024-01-15 INFO Fetch https://example.com/api/data done".to_string(),
        level: Some(LogLevel::Info),
        timestamp: Some("2024-01-15".to_string()),
        message: "Fetch https://example.com/api/data done".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    // The URL should be highlighted as a URL (blue), not as a path (cyan)
    let has_url = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Blue) && span.content.contains("https://"));
    assert!(has_url, "URLs should take priority over path highlighting");
}

// ---------------------------------------------------------------------------
// New pattern tests: numbers, keywords, pointers, unix processes, ipv6, dates
// ---------------------------------------------------------------------------

#[test]
fn test_number_highlighted() {
    let parsed = ParsedLine {
        raw: "Processed 150 records in 23ms".to_string(),
        level: None,
        timestamp: None,
        message: "Processed 150 records in 23ms".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_cyan_number = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Cyan) && span.content.as_ref() == "150");
    assert!(has_cyan_number, "Numbers should be highlighted in cyan");
}

#[test]
fn test_keyword_null_highlighted() {
    let parsed = ParsedLine {
        raw: "2024-01-15 ERROR Value was null for key".to_string(),
        level: Some(LogLevel::Error),
        timestamp: Some("2024-01-15".to_string()),
        message: "Value was null for key".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_keyword = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::LightRed) && span.content.as_ref() == "null");
    assert!(has_keyword, "null keyword should be highlighted");
}

#[test]
fn test_keyword_true_false_highlighted() {
    let parsed = ParsedLine {
        raw: "verbose=true debug=false".to_string(),
        level: None,
        timestamp: None,
        message: "verbose=true debug=false".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_true = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::LightRed) && span.content.as_ref() == "true");
    let has_false = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::LightRed) && span.content.as_ref() == "false");
    assert!(has_true, "true keyword should be highlighted");
    assert!(has_false, "false keyword should be highlighted");
}

#[test]
fn test_pointer_address_highlighted() {
    let parsed = ParsedLine {
        raw: "Segfault at address 0x7fff5fbff8c0 in thread 3".to_string(),
        level: None,
        timestamp: None,
        message: "Segfault at address 0x7fff5fbff8c0 in thread 3".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_pointer = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Indexed(208)) && span.content.contains("0x7fff"));
    assert!(has_pointer, "Pointer addresses should be highlighted");
}

#[test]
fn test_unix_process_highlighted() {
    let parsed = ParsedLine {
        raw: "Jan 15 08:30:01 myhost sshd[1234]: Accepted publickey".to_string(),
        level: None,
        timestamp: Some("Jan 15 08:30:01".to_string()),
        message: "sshd[1234]: Accepted publickey".to_string(),
        format: LogFormat::Syslog,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_process = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::Blue) && span.content.contains("sshd[1234]"));
    assert!(
        has_process,
        "Unix processes (name[pid]) should be highlighted"
    );
}

#[test]
fn test_inline_date_highlighted() {
    let parsed = ParsedLine {
        raw: "Backup completed for date 2024-06-15T10:30:00Z successfully".to_string(),
        level: None,
        timestamp: None,
        message: "Backup completed for date 2024-06-15T10:30:00Z successfully".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let has_date = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::DarkGray) && span.content.contains("2024-06-15"));
    assert!(has_date, "Inline dates should be highlighted in dark gray");
}

// ---------------------------------------------------------------------------
// Extra fields rendering tests
// ---------------------------------------------------------------------------

#[test]
fn test_json_extra_fields_rendered_dimmed() {
    let parsed = ParsedLine {
        raw: r#"{"level":"error","message":"Failed","error":"Connection refused"}"#.to_string(),
        level: Some(LogLevel::Error),
        timestamp: None,
        message: "Failed".to_string(),
        format: LogFormat::Json,
        pretty_json: None,
        extra_fields: vec![("error".to_string(), r#""Connection refused""#.to_string())],
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let text: String = styled.spans.iter().map(|s| s.content.to_string()).collect();
    assert!(
        text.contains("error="),
        "Extra fields should appear in output: {text}"
    );
    // The two-space separator before extra fields should be DarkGray+DIM
    let has_separator = styled
        .spans
        .iter()
        .any(|span| span.style.fg == Some(Color::DarkGray) && span.content.as_ref() == "  ");
    assert!(has_separator, "Extra fields should have a dimmed separator");
}

#[test]
fn test_json_no_extra_fields_no_trailing_space() {
    let parsed = ParsedLine {
        raw: r#"{"level":"info","message":"Clean line"}"#.to_string(),
        level: Some(LogLevel::Info),
        timestamp: None,
        message: "Clean line".to_string(),
        format: LogFormat::Json,
        pretty_json: None,
        extra_fields: Vec::new(),
        template: String::new(),
    };
    let styled = highlight_line(&parsed);
    let text: String = styled.spans.iter().map(|s| s.content.to_string()).collect();
    assert!(
        !text.ends_with("  "),
        "No trailing separator when extra_fields is empty"
    );
}

// ---------------------------------------------------------------------------
// Search match highlighting tests (apply_search_highlight)
// ---------------------------------------------------------------------------

#[test]
fn test_search_highlight_single_span() {
    let line = Line::from(vec![Span::styled(
        "Connection refused".to_string(),
        Style::default().fg(Color::Red),
    )]);
    let result = apply_search_highlight(line, "connect");
    let highlight = Style::default().bg(Color::Yellow).fg(Color::Black);
    assert_eq!(result.spans.len(), 2);
    assert_eq!(result.spans[0].content.as_ref(), "Connect");
    assert_eq!(result.spans[0].style, highlight);
    assert_eq!(result.spans[1].content.as_ref(), "ion refused");
    assert_eq!(result.spans[1].style, Style::default().fg(Color::Red));
}

#[test]
fn test_search_highlight_cross_span_boundary() {
    let line = Line::from(vec![
        Span::styled("Con".to_string(), Style::default().fg(Color::Red)),
        Span::styled("nect".to_string(), Style::default().fg(Color::Cyan)),
    ]);
    let result = apply_search_highlight(line, "connect");
    let highlight = Style::default().bg(Color::Yellow).fg(Color::Black);
    assert_eq!(result.spans.len(), 2);
    assert_eq!(result.spans[0].content.as_ref(), "Con");
    assert_eq!(result.spans[0].style, highlight);
    assert_eq!(result.spans[1].content.as_ref(), "nect");
    assert_eq!(result.spans[1].style, highlight);
}

#[test]
fn test_search_highlight_no_match() {
    let line = Line::from(vec![Span::styled(
        "Hello world".to_string(),
        Style::default().fg(Color::Green),
    )]);
    let result = apply_search_highlight(line, "xyz");
    assert_eq!(result.spans.len(), 1);
    assert_eq!(result.spans[0].content.as_ref(), "Hello world");
}

#[test]
fn test_search_highlight_empty_pattern() {
    let line = Line::from(vec![Span::styled("Hello".to_string(), Style::default())]);
    let result = apply_search_highlight(line, "");
    assert_eq!(result.spans.len(), 1);
}

#[test]
fn test_search_highlight_multiple_matches() {
    let line = Line::from(vec![Span::styled(
        "error: got error again".to_string(),
        Style::default().fg(Color::Red),
    )]);
    let result = apply_search_highlight(line, "error");
    let highlight = Style::default().bg(Color::Yellow).fg(Color::Black);
    // "error" + ": got " + "error" + " again" = 4 spans
    assert_eq!(result.spans.len(), 4);
    assert_eq!(result.spans[0].style, highlight);
    assert_eq!(result.spans[2].style, highlight);
}

#[test]
fn test_search_highlight_case_insensitive() {
    let line = Line::from(vec![Span::styled(
        "ERROR occurred".to_string(),
        Style::default().fg(Color::Red),
    )]);
    let result = apply_search_highlight(line, "error");
    let highlight = Style::default().bg(Color::Yellow).fg(Color::Black);
    assert_eq!(result.spans[0].content.as_ref(), "ERROR");
    assert_eq!(result.spans[0].style, highlight);
}
