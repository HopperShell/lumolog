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
        extra_fields: Vec::new(),
    }
}

#[test]
fn test_empty_pattern_returns_all() {
    let lines = vec![make_line("line one", None), make_line("line two", None)];
    let result = filter_lines(&lines, "", None).indices;
    assert_eq!(result.len(), 2);
}

#[test]
fn test_case_insensitive_match() {
    let lines = vec![
        make_line("ERROR something broke", Some(LogLevel::Error)),
        make_line("INFO all good", Some(LogLevel::Info)),
        make_line("error again", Some(LogLevel::Error)),
    ];
    let result = filter_lines(&lines, "error", None).indices;
    assert_eq!(result.len(), 2);
}

#[test]
fn test_no_matches() {
    let lines = vec![
        make_line("INFO all good", Some(LogLevel::Info)),
        make_line("DEBUG tracing", Some(LogLevel::Debug)),
    ];
    let result = filter_lines(&lines, "FATAL", None).indices;
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
    let result = filter_lines(&lines, "", Some(LogLevel::Warn)).indices;
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
    let result = filter_lines(&lines, "", Some(LogLevel::Error)).indices;
    assert_eq!(result, vec![2, 3]);
}

#[test]
fn test_level_filter_none_shows_all() {
    let lines = vec![
        make_line("DEBUG stuff", Some(LogLevel::Debug)),
        make_line("ERROR bad", Some(LogLevel::Error)),
    ];
    let result = filter_lines(&lines, "", None).indices;
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
    let result = filter_lines(&lines, "user", Some(LogLevel::Warn)).indices;
    assert_eq!(result, vec![1, 2]);
}

#[test]
fn test_level_filter_unclassified_lines_always_shown() {
    let lines = vec![
        make_line("DEBUG low level", Some(LogLevel::Debug)),
        make_line("--- separator ---", None),
        make_line("ERROR failure", Some(LogLevel::Error)),
    ];
    let result = filter_lines(&lines, "", Some(LogLevel::Error)).indices;
    // Unclassified line (None) passes through, Debug is filtered out
    assert_eq!(result, vec![1, 2]);
}

#[test]
fn test_filter_result_exact_mode() {
    let lines = vec![
        make_line("ERROR something broke", Some(LogLevel::Error)),
        make_line("INFO all good", Some(LogLevel::Info)),
    ];
    let result = filter_lines(&lines, "ERROR", None);
    assert!(!result.is_fuzzy);
    assert_eq!(result.indices, vec![0]);
}

#[test]
fn test_filter_result_empty_pattern_not_fuzzy() {
    let lines = vec![make_line("line one", None), make_line("line two", None)];
    let result = filter_lines(&lines, "", None);
    assert!(!result.is_fuzzy);
    assert_eq!(result.indices.len(), 2);
}

// ---------------------------------------------------------------------------
// Fuzzy fallback tests
// ---------------------------------------------------------------------------

#[test]
fn test_fuzzy_fallback_on_zero_exact_matches() {
    let lines = vec![
        make_line("connection refused by remote host", Some(LogLevel::Error)),
        make_line("INFO healthy heartbeat", Some(LogLevel::Info)),
        make_line("WARN connection timeout", Some(LogLevel::Warn)),
    ];
    let result = filter_lines(&lines, "conref", None);
    assert!(result.is_fuzzy);
    assert!(result.indices.contains(&0));
}

#[test]
fn test_exact_match_preferred_over_fuzzy() {
    let lines = vec![
        make_line("connection refused", Some(LogLevel::Error)),
        make_line("INFO connected", Some(LogLevel::Info)),
    ];
    let result = filter_lines(&lines, "connection", None);
    assert!(!result.is_fuzzy);
    assert_eq!(result.indices, vec![0]);
}

#[test]
fn test_fuzzy_fallback_respects_level_filter() {
    let lines = vec![
        make_line("connection refused by remote host", Some(LogLevel::Info)),
        make_line("connection refused again", Some(LogLevel::Error)),
    ];
    let result = filter_lines(&lines, "conref", Some(LogLevel::Error));
    assert!(result.is_fuzzy);
    assert_eq!(result.indices, vec![1]);
}

#[test]
fn test_fuzzy_preserves_chronological_order() {
    let lines = vec![
        make_line("alpha bravo charlie", None),
        make_line("something else entirely", None),
        make_line("apple banana cherry", None),
    ];
    let result = filter_lines(&lines, "abc", None);
    assert!(result.is_fuzzy);
    let mut sorted = result.indices.clone();
    sorted.sort();
    assert_eq!(result.indices, sorted);
}

#[test]
fn test_fuzzy_no_matches_returns_empty() {
    let lines = vec![
        make_line("INFO all good", Some(LogLevel::Info)),
        make_line("DEBUG tracing", Some(LogLevel::Debug)),
    ];
    let result = filter_lines(&lines, "zzzzz", None);
    assert!(result.indices.is_empty());
}
