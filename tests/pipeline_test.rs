mod test_helpers;

use lumolog::parser::{LogFormat, LogLevel};
use ratatui::style::Color;
use test_helpers::*;

// ===========================================================================
// Format detection tests — every testdata file should be detected correctly
// ===========================================================================

#[test]
fn test_pipeline_detect_json() {
    let result = pipeline("testdata/sample_json.log");
    assert_eq!(result.format, LogFormat::Json);
}

#[test]
fn test_pipeline_detect_syslog() {
    let result = pipeline("testdata/sample_syslog.log");
    assert_eq!(result.format, LogFormat::Syslog);
}

#[test]
fn test_pipeline_detect_plain() {
    let result = pipeline("testdata/sample_plain.log");
    assert_eq!(result.format, LogFormat::Plain);
}

#[test]
fn test_pipeline_detect_logfmt() {
    let result = pipeline("testdata/sample_logfmt.log");
    assert_eq!(result.format, LogFormat::Logfmt);
}

#[test]
fn test_pipeline_detect_klog() {
    let result = pipeline("testdata/sample_klog.log");
    assert_eq!(result.format, LogFormat::Klog);
}

#[test]
fn test_pipeline_detect_log4j() {
    let result = pipeline("testdata/sample_log4j.log");
    assert_eq!(result.format, LogFormat::Log4j);
}

#[test]
fn test_pipeline_detect_python() {
    let result = pipeline("testdata/sample_python.log");
    assert_eq!(result.format, LogFormat::PythonLog);
}

#[test]
fn test_pipeline_detect_apache() {
    let result = pipeline("testdata/sample_apache.log");
    assert_eq!(result.format, LogFormat::AccessLog);
}

#[test]
fn test_pipeline_detect_docker() {
    let result = pipeline("testdata/sample_docker.log");
    assert_eq!(result.format, LogFormat::Json);
}

#[test]
fn test_pipeline_detect_pino() {
    let result = pipeline("testdata/sample_pino.log");
    assert_eq!(result.format, LogFormat::Json);
}

#[test]
fn test_pipeline_detect_mixed_as_plain() {
    let result = pipeline("testdata/sample_mixed.log");
    assert_eq!(result.format, LogFormat::Plain);
}

// ===========================================================================
// JSON log: full pipeline (levels, colors, extra fields)
// ===========================================================================

#[test]
fn test_json_pipeline_levels() {
    let result = pipeline("testdata/sample_json.log");
    let expected = vec![
        Some(LogLevel::Info),
        Some(LogLevel::Debug),
        Some(LogLevel::Warn),
        Some(LogLevel::Error),
        Some(LogLevel::Info),
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_json_pipeline_colors() {
    let result = pipeline("testdata/sample_json.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

#[test]
fn test_json_pipeline_timestamps() {
    let result = pipeline("testdata/sample_json.log");
    for (i, parsed) in result.parsed.iter().enumerate() {
        assert!(
            parsed.timestamp.is_some(),
            "Line {}: JSON log should have timestamp",
            i
        );
    }
}

#[test]
fn test_json_pipeline_extra_fields() {
    let result = pipeline("testdata/sample_json.log");

    // Line 0: has "service" extra field
    assert!(
        result.parsed[0]
            .extra_fields
            .iter()
            .any(|(k, _)| k == "service"),
        "Line 0 should have 'service' extra field"
    );

    // Line 3 (error): has "error" and "host" extra fields
    assert!(
        result.parsed[3]
            .extra_fields
            .iter()
            .any(|(k, _)| k == "error"),
        "Line 3 should have 'error' extra field"
    );
    assert!(
        result.parsed[3]
            .extra_fields
            .iter()
            .any(|(k, _)| k == "host"),
        "Line 3 should have 'host' extra field"
    );
}

#[test]
fn test_json_extra_fields_rendered() {
    let result = pipeline("testdata/sample_json.log");
    // Line 3 (error line): highlighted output should contain "error=" from extra fields
    let text = line_text(&result.highlighted[3]);
    assert!(
        text.contains("error="),
        "JSON extra fields should appear in highlighted output: {text}"
    );
}

// ===========================================================================
// Plain log: full pipeline
// ===========================================================================

#[test]
fn test_plain_pipeline_levels() {
    let result = pipeline("testdata/sample_plain.log");
    let expected = vec![
        Some(LogLevel::Info),  // INFO  Application starting up
        Some(LogLevel::Debug), // DEBUG Loading configuration
        Some(LogLevel::Info),  // INFO  Connected to database
        Some(LogLevel::Warn),  // WARN  Cache miss rate
        Some(LogLevel::Error), // ERROR Failed to connect
        Some(LogLevel::Info),  // INFO  Falling back
        Some(LogLevel::Debug), // DEBUG Request received
        Some(LogLevel::Info),  // INFO  Response sent
        Some(LogLevel::Warn),  // WARN  Slow query
        Some(LogLevel::Error), // ERROR Unhandled exception
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_plain_pipeline_colors() {
    let result = pipeline("testdata/sample_plain.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

#[test]
fn test_plain_timestamps_in_gray() {
    let result = pipeline("testdata/sample_plain.log");
    for (i, line) in result.highlighted.iter().enumerate() {
        assert!(
            has_span(line, "2024-01-15", Color::DarkGray),
            "Line {}: timestamp should be dark gray. Spans: {}",
            i,
            debug_spans(line)
        );
    }
}

#[test]
fn test_plain_pattern_highlighting() {
    let result = pipeline("testdata/sample_plain.log");

    // Line 1: "/etc/app/config.yaml" path should be cyan
    assert!(
        has_span(&result.highlighted[1], "/etc/app/config.yaml", Color::Cyan),
        "Line 1: file path should be cyan. Spans: {}",
        debug_spans(&result.highlighted[1])
    );

    // Line 6: "GET" HTTP method should be magenta
    assert!(
        has_span(&result.highlighted[6], "GET", Color::Magenta),
        "Line 6: HTTP method should be magenta. Spans: {}",
        debug_spans(&result.highlighted[6])
    );
}

// ===========================================================================
// Syslog: full pipeline
// ===========================================================================

#[test]
fn test_syslog_pipeline_timestamps() {
    let result = pipeline("testdata/sample_syslog.log");
    for (i, parsed) in result.parsed.iter().enumerate() {
        assert!(
            parsed.timestamp.is_some(),
            "Line {}: syslog should have timestamp",
            i
        );
    }
}

#[test]
fn test_syslog_pattern_highlighting() {
    let result = pipeline("testdata/sample_syslog.log");

    // Line 0: "sshd[1234]" process should be blue
    assert!(
        has_span(&result.highlighted[0], "sshd[1234]", Color::Blue),
        "Line 0: unix process should be blue. Spans: {}",
        debug_spans(&result.highlighted[0])
    );

    // Line 0: IP address should be cyan
    assert!(
        has_span(&result.highlighted[0], "192.168.1.100", Color::Cyan),
        "Line 0: IP address should be cyan. Spans: {}",
        debug_spans(&result.highlighted[0])
    );
}

// ===========================================================================
// Logfmt: full pipeline
// ===========================================================================

#[test]
fn test_logfmt_pipeline_levels() {
    let result = pipeline("testdata/sample_logfmt.log");
    let expected = vec![
        Some(LogLevel::Info),
        Some(LogLevel::Debug),
        Some(LogLevel::Warn),
        Some(LogLevel::Error),
        Some(LogLevel::Info),
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_logfmt_pipeline_colors() {
    let result = pipeline("testdata/sample_logfmt.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

#[test]
fn test_logfmt_pipeline_messages() {
    let result = pipeline("testdata/sample_logfmt.log");
    assert_eq!(result.parsed[0].message, "server starting");
    assert_eq!(result.parsed[1].message, "connected to database");
    assert_eq!(result.parsed[2].message, "cache miss rate high");
    assert_eq!(result.parsed[3].message, "connection refused");
    assert_eq!(result.parsed[4].message, "request handled");
}

// ===========================================================================
// Klog: full pipeline
// ===========================================================================

#[test]
fn test_klog_pipeline_levels() {
    let result = pipeline("testdata/sample_klog.log");
    let expected = vec![
        Some(LogLevel::Info),  // I
        Some(LogLevel::Info),  // I
        Some(LogLevel::Warn),  // W
        Some(LogLevel::Info),  // I
        Some(LogLevel::Error), // E
        Some(LogLevel::Info),  // I
        Some(LogLevel::Warn),  // W
        Some(LogLevel::Error), // E
        Some(LogLevel::Fatal), // F
        Some(LogLevel::Info),  // I
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_klog_pipeline_colors() {
    let result = pipeline("testdata/sample_klog.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

// ===========================================================================
// Log4j: full pipeline
// ===========================================================================

#[test]
fn test_log4j_pipeline_levels() {
    let result = pipeline("testdata/sample_log4j.log");
    let expected = vec![
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Debug), // DEBUG
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Warn),  // WARN
        Some(LogLevel::Error), // ERROR
        Some(LogLevel::Debug), // DEBUG
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Error), // ERROR
        Some(LogLevel::Fatal), // FATAL
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_log4j_pipeline_colors() {
    let result = pipeline("testdata/sample_log4j.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

#[test]
fn test_log4j_extra_fields() {
    let result = pipeline("testdata/sample_log4j.log");
    // Every line should have thread and class extra fields
    for (i, parsed) in result.parsed.iter().enumerate() {
        assert!(
            parsed.extra_fields.iter().any(|(k, _)| k == "thread"),
            "Line {}: log4j should have 'thread' extra field",
            i
        );
        assert!(
            parsed.extra_fields.iter().any(|(k, _)| k == "class"),
            "Line {}: log4j should have 'class' extra field",
            i
        );
    }
}

// ===========================================================================
// Python log: full pipeline
// ===========================================================================

#[test]
fn test_python_pipeline_levels() {
    let result = pipeline("testdata/sample_python.log");
    let expected = vec![
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Debug), // DEBUG
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Warn),  // WARNING
        Some(LogLevel::Error), // ERROR
        Some(LogLevel::Debug), // DEBUG
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Warn),  // WARNING
        Some(LogLevel::Error), // ERROR
        Some(LogLevel::Fatal), // CRITICAL
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_python_pipeline_colors() {
    let result = pipeline("testdata/sample_python.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

#[test]
fn test_python_module_extra_fields() {
    let result = pipeline("testdata/sample_python.log");
    for (i, parsed) in result.parsed.iter().enumerate() {
        assert!(
            parsed.extra_fields.iter().any(|(k, _)| k == "module"),
            "Line {}: python log should have 'module' extra field",
            i
        );
    }
}

// ===========================================================================
// Apache access log: full pipeline
// ===========================================================================

#[test]
fn test_apache_pipeline_levels() {
    let result = pipeline("testdata/sample_apache.log");
    let expected = vec![
        Some(LogLevel::Info),  // 200
        Some(LogLevel::Warn),  // 401
        Some(LogLevel::Info),  // 204
        Some(LogLevel::Info),  // 304
        Some(LogLevel::Error), // 500
        Some(LogLevel::Info),  // 200 (combined)
        Some(LogLevel::Info),  // 200 (combined)
        Some(LogLevel::Warn),  // 404 (combined)
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_apache_pipeline_colors() {
    let result = pipeline("testdata/sample_apache.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

#[test]
fn test_apache_messages_contain_method_and_status() {
    let result = pipeline("testdata/sample_apache.log");
    assert!(result.parsed[0].message.contains("GET"));
    assert!(result.parsed[0].message.contains("200"));
    assert!(result.parsed[1].message.contains("POST"));
    assert!(result.parsed[1].message.contains("401"));
    assert!(result.parsed[4].message.contains("PUT"));
    assert!(result.parsed[4].message.contains("500"));
}

// ===========================================================================
// Docker JSON logs: full pipeline
// ===========================================================================

#[test]
fn test_docker_pipeline_levels() {
    let result = pipeline("testdata/sample_docker.log");
    assert_level(&result.parsed[0], None, 0); // plain startup
    assert_level(&result.parsed[1], Some(LogLevel::Info), 1);
    assert_level(&result.parsed[3], Some(LogLevel::Warn), 3);
    assert_level(&result.parsed[5], Some(LogLevel::Error), 5);
    assert_level(&result.parsed[6], Some(LogLevel::Fatal), 6); // panic
}

#[test]
fn test_docker_pipeline_colors() {
    let result = pipeline("testdata/sample_docker.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

#[test]
fn test_docker_no_double_timestamp() {
    let result = pipeline("testdata/sample_docker.log");
    // Line 1: had "2024-01-15T08:30:01Z INFO  Listening on 0.0.0.0:3000" as message
    // After fix: embedded timestamp should be stripped, message starts with "INFO"
    let msg = &result.parsed[1].message;
    assert!(
        !msg.starts_with("2024"),
        "Docker message should not start with embedded timestamp: {msg:?}"
    );
    // Wrapper timestamp should still be present
    assert!(
        result.parsed[1].timestamp.is_some(),
        "Docker line should have wrapper timestamp"
    );
}

#[test]
fn test_docker_no_embedded_timestamp_in_highlight() {
    let result = pipeline("testdata/sample_docker.log");
    // Line 1: the highlighted output should only show one timestamp (the wrapper)
    let text = line_text(&result.highlighted[1]);
    // Count how many times a date-like pattern appears
    let ts_count = text.matches("2024-01-15").count();
    assert!(
        ts_count <= 1,
        "Docker line should show at most 1 timestamp, found {ts_count}: {text}"
    );
}

#[test]
fn test_docker_plain_message_not_stripped() {
    let result = pipeline("testdata/sample_docker.log");
    // Line 0: "Starting myapp v2.4.1" has no embedded timestamp — should be unchanged
    assert_eq!(result.parsed[0].message, "Starting myapp v2.4.1");
}

// ===========================================================================
// Pino (numeric levels): full pipeline
// ===========================================================================

#[test]
fn test_pino_pipeline_levels() {
    let result = pipeline("testdata/sample_pino.log");
    let expected = vec![
        Some(LogLevel::Info),  // 30
        Some(LogLevel::Debug), // 20
        Some(LogLevel::Warn),  // 40
        Some(LogLevel::Error), // 50
        Some(LogLevel::Fatal), // 60
        Some(LogLevel::Trace), // 10
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_pino_pipeline_colors() {
    let result = pipeline("testdata/sample_pino.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

#[test]
fn test_pino_epoch_timestamps_resolved() {
    let result = pipeline("testdata/sample_pino.log");
    // Pino uses epoch millis (e.g. 1705302601000) — should now resolve to ISO strings
    for (i, parsed) in result.parsed.iter().enumerate() {
        assert!(
            parsed.timestamp.is_some(),
            "Line {}: Pino epoch timestamp should be resolved to a string",
            i
        );
    }
}

#[test]
fn test_pino_epoch_timestamp_is_readable() {
    let result = pipeline("testdata/sample_pino.log");
    // Line 0: time=1705302601000 → should be 2024-01-15T...
    let ts = result.parsed[0].timestamp.as_ref().unwrap();
    assert!(
        ts.starts_with("2024-01-15"),
        "Pino epoch 1705302601000 should resolve to 2024-01-15, got: {ts}"
    );
}

#[test]
fn test_pino_timestamps_displayed_in_highlight() {
    let result = pipeline("testdata/sample_pino.log");
    // Line 0: highlighted output should now include the resolved timestamp
    let text = line_text(&result.highlighted[0]);
    assert!(
        text.contains("2024-01-15"),
        "Pino highlighted output should show resolved timestamp: {text}"
    );
    // Timestamp should be in DarkGray
    assert!(
        has_span(&result.highlighted[0], "2024-01-15", Color::DarkGray),
        "Pino timestamp should be DarkGray. Spans: {}",
        debug_spans(&result.highlighted[0])
    );
}

// ===========================================================================
// Mixed plain log: full pipeline
// ===========================================================================

#[test]
fn test_mixed_pipeline_levels() {
    let result = pipeline("testdata/sample_mixed.log");
    let expected = vec![
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Debug), // DEBUG
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Warn),  // WARN
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Error), // ERROR
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Debug), // DEBUG
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Warn),  // WARN
        Some(LogLevel::Info),  // INFO
        Some(LogLevel::Error), // ERROR
        Some(LogLevel::Fatal), // FATAL
    ];
    for (i, exp) in expected.iter().enumerate() {
        assert_level(&result.parsed[i], *exp, i);
    }
}

#[test]
fn test_mixed_pipeline_colors() {
    let result = pipeline("testdata/sample_mixed.log");
    for (i, (parsed, line)) in result.parsed.iter().zip(result.highlighted.iter()).enumerate() {
        assert_level_color(parsed, line, i);
    }
}

// ===========================================================================
// Pattern highlighting across formats
// ===========================================================================

#[test]
fn test_mixed_ip_addresses_cyan() {
    let result = pipeline("testdata/sample_mixed.log");
    // Line 2: "10.0.0.50:5432" — IP should be cyan
    assert!(
        has_span(&result.highlighted[2], "10.0.0.50", Color::Cyan),
        "Line 2: IP address should be cyan. Spans: {}",
        debug_spans(&result.highlighted[2])
    );
}

#[test]
fn test_mixed_uuid_magenta() {
    let result = pipeline("testdata/sample_mixed.log");
    // Line 7: UUID should be magenta
    assert!(
        has_span(&result.highlighted[7], "f47ac10b", Color::Magenta),
        "Line 7: UUID should be magenta. Spans: {}",
        debug_spans(&result.highlighted[7])
    );
}

#[test]
fn test_mixed_url_blue() {
    let result = pipeline("testdata/sample_mixed.log");
    // Line 10: URL should be blue
    assert!(
        has_span(&result.highlighted[10], "https://", Color::Blue),
        "Line 10: URL should be blue. Spans: {}",
        debug_spans(&result.highlighted[10])
    );
}

#[test]
fn test_mixed_http_method_magenta() {
    let result = pipeline("testdata/sample_mixed.log");
    // Line 4: "GET" should be magenta
    assert!(
        has_span(&result.highlighted[4], "GET", Color::Magenta),
        "Line 4: HTTP method should be magenta. Spans: {}",
        debug_spans(&result.highlighted[4])
    );
    // Line 8: "POST" should be magenta
    assert!(
        has_span(&result.highlighted[8], "POST", Color::Magenta),
        "Line 8: HTTP method should be magenta. Spans: {}",
        debug_spans(&result.highlighted[8])
    );
}

#[test]
fn test_mixed_path_cyan() {
    let result = pipeline("testdata/sample_mixed.log");
    // Line 1: "/etc/app/config.yaml" should be cyan
    assert!(
        has_span(&result.highlighted[1], "/etc/app/config.yaml", Color::Cyan),
        "Line 1: file path should be cyan. Spans: {}",
        debug_spans(&result.highlighted[1])
    );
}

#[test]
fn test_mixed_quoted_string_yellow() {
    let result = pipeline("testdata/sample_mixed.log");
    // Line 5: quoted "Connection refused" should be yellow
    assert!(
        has_span(
            &result.highlighted[5],
            "Connection refused",
            Color::Yellow
        ),
        "Line 5: quoted string should be yellow. Spans: {}",
        debug_spans(&result.highlighted[5])
    );
}

// ===========================================================================
// Highlighting improvements: version numbers, debug colors, badges, numbers, HTTP
// ===========================================================================

#[test]
fn test_version_number_single_span() {
    // "2.4.1" should stay as one span, not be split into "2.4" + "." + "1"
    let result = pipeline("testdata/sample_mixed.log");
    // Line 0: "version 2.4.1"
    let text = line_text(&result.highlighted[0]);
    assert!(text.contains("2.4.1"), "Line 0 should contain version string");
    // The version should be a single span colored Cyan (number_style)
    assert!(
        has_span(&result.highlighted[0], "2.4.1", Color::Cyan),
        "Version 2.4.1 should be a single cyan span. Spans: {}",
        debug_spans(&result.highlighted[0])
    );
}

#[test]
fn test_debug_line_distinguishable_from_timestamp() {
    // Debug level should use Indexed(249), not DarkGray like timestamps
    let result = pipeline("testdata/sample_plain.log");
    // Line 1 is DEBUG
    assert_level(&result.parsed[1], Some(LogLevel::Debug), 1);
    // Timestamp should be DarkGray
    assert!(
        has_span(&result.highlighted[1], "2024-01-15", Color::DarkGray),
        "Timestamp should still be DarkGray"
    );
    // The rest of the line (level + message) should use Indexed(249), not DarkGray
    assert!(
        has_color(&result.highlighted[1], Color::Indexed(249)),
        "Debug line should use Indexed(249) for message. Spans: {}",
        debug_spans(&result.highlighted[1])
    );
}

#[test]
fn test_trace_line_distinguishable_from_timestamp() {
    // Trace level should use Indexed(243)
    let result = pipeline("testdata/sample_pino.log");
    // Line 5 is Trace (level 10)
    assert_level(&result.parsed[5], Some(LogLevel::Trace), 5);
    assert!(
        has_color(&result.highlighted[5], Color::Indexed(243)),
        "Trace line should use Indexed(243). Spans: {}",
        debug_spans(&result.highlighted[5])
    );
}

#[test]
fn test_plain_level_keyword_is_bold() {
    // In plain format, the level keyword should be bold
    let result = pipeline("testdata/sample_plain.log");
    // Line 0: INFO
    assert!(
        has_span_with_modifier(
            &result.highlighted[0],
            "INFO",
            Color::Green,
            ratatui::style::Modifier::BOLD,
        ),
        "Plain format INFO should be bold+green. Spans: {}",
        debug_spans(&result.highlighted[0])
    );
    // Line 4: ERROR
    assert!(
        has_span_with_modifier(
            &result.highlighted[4],
            "ERROR",
            Color::Red,
            ratatui::style::Modifier::BOLD,
        ),
        "Plain format ERROR should be bold+red. Spans: {}",
        debug_spans(&result.highlighted[4])
    );
}

#[test]
fn test_single_digit_not_highlighted() {
    // "thread #3" — the "3" should NOT be highlighted as a number
    let result = pipeline("testdata/sample_plain.log");
    // Line 9: "worker thread #3"
    let line = &result.highlighted[9];
    // The "3" should not be in a Cyan span by itself
    let has_cyan_3 = line
        .spans
        .iter()
        .any(|s| s.content.as_ref().trim() == "3" && s.style.fg == Some(Color::Cyan));
    assert!(
        !has_cyan_3,
        "Single digit '3' should not be highlighted as a number. Spans: {}",
        debug_spans(line)
    );
}

#[test]
fn test_http_protocol_version_not_number_highlighted() {
    // "HTTP/1.1" — the "1.1" should not be highlighted as a separate number
    let result = pipeline_from_lines(&[
        r#"2024-01-01 INFO GET /api/users HTTP/1.1 200 OK"#,
        r#"2024-01-01 INFO GET /api/users HTTP/1.1 200 OK"#,
    ]);
    let line = &result.highlighted[0];
    let text = line_text(line);
    assert!(text.contains("HTTP/1.1"), "Should contain HTTP/1.1");
    // "1.1" should NOT be in a Cyan span by itself
    let has_cyan_11 = line.spans.iter().any(|s| {
        s.content.as_ref() == "1.1" && s.style.fg == Some(Color::Cyan)
    });
    assert!(
        !has_cyan_11,
        "HTTP/1.1 version should not have '1.1' as separate cyan span. Spans: {}",
        debug_spans(line)
    );
}

#[test]
fn test_multidigit_numbers_still_highlighted() {
    // Numbers with 2+ digits should still be highlighted
    let result = pipeline("testdata/sample_plain.log");
    // Line 7: "200 OK (23ms)" — 200 should be cyan
    assert!(
        has_span(&result.highlighted[7], "200", Color::Cyan),
        "Multi-digit number 200 should be cyan. Spans: {}",
        debug_spans(&result.highlighted[7])
    );
}

#[test]
fn test_number_with_unit_still_highlighted() {
    // Numbers with units should still be highlighted even if single digit
    let result = pipeline_from_lines(&["2024-01-01 INFO response time 5ms"]);
    let line = &result.highlighted[0];
    assert!(
        has_span(line, "5ms", Color::Cyan),
        "Number with unit '5ms' should be cyan. Spans: {}",
        debug_spans(line)
    );
}

// ===========================================================================
// pipeline_from_lines: ad-hoc tests without files
// ===========================================================================

#[test]
fn test_adhoc_json_lines() {
    let result = pipeline_from_lines(&[
        r#"{"level":"error","message":"disk full","timestamp":"2024-01-01T00:00:00Z"}"#,
        r#"{"level":"info","message":"recovered","timestamp":"2024-01-01T00:00:01Z"}"#,
    ]);
    assert_eq!(result.format, LogFormat::Json);
    assert_level(&result.parsed[0], Some(LogLevel::Error), 0);
    assert_level(&result.parsed[1], Some(LogLevel::Info), 1);
    assert_error_is_red(&result.highlighted[0], 0);
    assert_info_is_green(&result.highlighted[1], 1);
}

#[test]
fn test_adhoc_plain_lines() {
    let result = pipeline_from_lines(&[
        "2024-01-01 00:00:00 WARN  disk space low",
        "2024-01-01 00:00:01 ERROR disk full",
    ]);
    assert_eq!(result.format, LogFormat::Plain);
    assert_warn_is_yellow(&result.highlighted[0], 0);
    assert_error_is_red(&result.highlighted[1], 1);
}

// ===========================================================================
// debug_spans helper verification
// ===========================================================================

#[test]
fn test_debug_spans_produces_readable_output() {
    let result = pipeline_from_lines(&["2024-01-01 ERROR something broke"]);
    let debug = debug_spans(&result.highlighted[0]);
    // Should contain color names and quoted text
    assert!(debug.contains("Red"), "debug output should mention Red");
    assert!(
        debug.contains("DarkGray") || debug.contains("Indexed"),
        "debug output should mention a gray color for timestamp"
    );
}
