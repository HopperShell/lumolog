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
    let result = filter_lines(&lines, "", None);
    assert_eq!(result.len(), 2);
}

#[test]
fn test_case_insensitive_match() {
    let lines = vec![
        make_line("ERROR something broke", Some(LogLevel::Error)),
        make_line("INFO all good", Some(LogLevel::Info)),
        make_line("error again", Some(LogLevel::Error)),
    ];
    let result = filter_lines(&lines, "error", None);
    assert_eq!(result.len(), 2);
}

#[test]
fn test_no_matches() {
    let lines = vec![
        make_line("INFO all good", Some(LogLevel::Info)),
        make_line("DEBUG tracing", Some(LogLevel::Debug)),
    ];
    let result = filter_lines(&lines, "FATAL", None);
    assert_eq!(result.len(), 0);
}

// ---------------------------------------------------------------------------
// Level filtering tests
// ---------------------------------------------------------------------------

#[test]
fn test_level_filter_warn_and_above() {
    let lines = vec![
        make_line("DEBUG stuff", Some(LogLevel::Debug)),
        make_line("INFO ok", Some(LogLevel::Info)),
        make_line("WARN hmm", Some(LogLevel::Warn)),
        make_line("ERROR bad", Some(LogLevel::Error)),
        make_line("no level", None),
    ];
    let result = filter_lines(&lines, "", Some(LogLevel::Warn));
    // Should include Warn, Error, and the line with no level
    assert_eq!(result, vec![2, 3, 4]);
}

#[test]
fn test_level_filter_error_only() {
    let lines = vec![
        make_line("INFO ok", Some(LogLevel::Info)),
        make_line("WARN hmm", Some(LogLevel::Warn)),
        make_line("ERROR bad", Some(LogLevel::Error)),
        make_line("FATAL crash", Some(LogLevel::Fatal)),
    ];
    let result = filter_lines(&lines, "", Some(LogLevel::Error));
    assert_eq!(result, vec![2, 3]);
}

#[test]
fn test_level_filter_none_shows_all() {
    let lines = vec![
        make_line("DEBUG stuff", Some(LogLevel::Debug)),
        make_line("ERROR bad", Some(LogLevel::Error)),
    ];
    let result = filter_lines(&lines, "", None);
    assert_eq!(result, vec![0, 1]);
}

#[test]
fn test_level_filter_combined_with_text_filter() {
    let lines = vec![
        make_line("INFO user logged in", Some(LogLevel::Info)),
        make_line("WARN user session expiring", Some(LogLevel::Warn)),
        make_line("ERROR user auth failed", Some(LogLevel::Error)),
        make_line("DEBUG user cache hit", Some(LogLevel::Debug)),
    ];
    // Text filter "user" + level >= Warn
    let result = filter_lines(&lines, "user", Some(LogLevel::Warn));
    assert_eq!(result, vec![1, 2]);
}

#[test]
fn test_level_filter_unclassified_lines_always_shown() {
    let lines = vec![
        make_line("DEBUG low level", Some(LogLevel::Debug)),
        make_line("--- separator ---", None),
        make_line("ERROR failure", Some(LogLevel::Error)),
    ];
    let result = filter_lines(&lines, "", Some(LogLevel::Error));
    // Unclassified line (None) passes through, Debug is filtered out
    assert_eq!(result, vec![1, 2]);
}
