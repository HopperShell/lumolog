use lumolog::filter::filter_lines;
use lumolog::parser::{LogFormat, LogLevel, ParsedLine};

fn make_line(raw: &str, level: Option<LogLevel>) -> ParsedLine {
    ParsedLine {
        raw: raw.to_string(),
        level,
        timestamp: None,
        message: raw.to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
    }
}

#[test]
fn test_empty_pattern_returns_all() {
    let lines = vec![make_line("line one", None), make_line("line two", None)];
    let result = filter_lines(&lines, "");
    assert_eq!(result.len(), 2);
}

#[test]
fn test_case_insensitive_match() {
    let lines = vec![
        make_line("ERROR something broke", Some(LogLevel::Error)),
        make_line("INFO all good", Some(LogLevel::Info)),
        make_line("error again", Some(LogLevel::Error)),
    ];
    let result = filter_lines(&lines, "error");
    assert_eq!(result.len(), 2);
}

#[test]
fn test_no_matches() {
    let lines = vec![
        make_line("INFO all good", Some(LogLevel::Info)),
        make_line("DEBUG tracing", Some(LogLevel::Debug)),
    ];
    let result = filter_lines(&lines, "FATAL");
    assert_eq!(result.len(), 0);
}
