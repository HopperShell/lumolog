use lumolog::ai::{build_system_prompt, parse_ai_response};

#[test]
fn test_parse_full_response() {
    let json = r#"{"text": "auth", "min_level": "ERROR", "time_range": "last_30m"}"#;
    let resp = parse_ai_response(json).unwrap();
    assert_eq!(resp.text.as_deref(), Some("auth"));
    assert_eq!(resp.min_level.as_deref(), Some("ERROR"));
    assert_eq!(resp.time_range.as_deref(), Some("last_30m"));
}

#[test]
fn test_parse_partial_response() {
    let json = r#"{"min_level": "WARN"}"#;
    let resp = parse_ai_response(json).unwrap();
    assert!(resp.text.is_none());
    assert_eq!(resp.min_level.as_deref(), Some("WARN"));
    assert!(resp.time_range.is_none());
}

#[test]
fn test_parse_empty_response() {
    let json = r#"{}"#;
    let resp = parse_ai_response(json).unwrap();
    assert!(resp.text.is_none());
    assert!(resp.min_level.is_none());
    assert!(resp.time_range.is_none());
}

#[test]
fn test_parse_invalid_json() {
    let json = "not json at all";
    assert!(parse_ai_response(json).is_err());
}

#[test]
fn test_parse_response_with_markdown_fencing() {
    let json = "```json\n{\"text\": \"error\"}\n```";
    let resp = parse_ai_response(json).unwrap();
    assert_eq!(resp.text.as_deref(), Some("error"));
}

#[test]
fn test_build_system_prompt_with_fields() {
    let prompt = build_system_prompt(
        "JSON",
        &["timestamp".into(), "level".into(), "message".into()],
        Some("2026-03-27T00:00:00 to 2026-03-27T12:00:00"),
    );
    assert!(prompt.contains("Log format: JSON"));
    assert!(prompt.contains("timestamp, level, message"));
    assert!(prompt.contains("2026-03-27T00:00:00 to 2026-03-27T12:00:00"));
}

#[test]
fn test_build_system_prompt_no_fields_no_time() {
    let prompt = build_system_prompt("Plain", &[], None);
    assert!(prompt.contains("Log format: Plain"));
    assert!(prompt.contains("none detected"));
    assert!(!prompt.contains("Time range of log"));
}
