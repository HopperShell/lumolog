use lumolog::highlighter::highlight_line;
use lumolog::parser::{LogFormat, LogLevel, ParsedLine};
use ratatui::style::Color;

#[test]
fn test_error_line_has_red() {
    let parsed = ParsedLine {
        raw: "2024-01-15 ERROR something broke".to_string(),
        level: Some(LogLevel::Error),
        timestamp: Some("2024-01-15".to_string()),
        message: "something broke".to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
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
