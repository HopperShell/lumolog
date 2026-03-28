use lumolog::ai::{AiFilterResponse, build_system_prompt, parse_ai_response};
use lumolog::app::App;

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
        &[],
    );
    assert!(prompt.contains("Log format: JSON"));
    assert!(prompt.contains("timestamp, level, message"));
    assert!(prompt.contains("2026-03-27T00:00:00 to 2026-03-27T12:00:00"));
}

#[test]
fn test_build_system_prompt_no_fields_no_time() {
    let prompt = build_system_prompt("Plain", &[], None, &[]);
    assert!(prompt.contains("Log format: Plain"));
    assert!(prompt.contains("none detected"));
    assert!(!prompt.contains("Time range of log"));
}

#[test]
fn test_build_system_prompt_with_sample_lines() {
    let samples = vec![
        r#"{"level":"ERROR","message":"Payment declined: card ending 4242"}"#.to_string(),
        r#"{"level":"INFO","message":"Cache warmup completed"}"#.to_string(),
    ];
    let prompt = build_system_prompt("JSON", &[], None, &samples);
    assert!(prompt.contains("Payment declined"));
    assert!(prompt.contains("Cache warmup"));
    assert!(prompt.contains("sample lines"));
}

#[test]
fn test_apply_ai_filter_text_only() {
    let lines = vec![
        "2026-03-27T10:00:00 INFO auth login successful".to_string(),
        "2026-03-27T10:00:01 ERROR auth login failed".to_string(),
        "2026-03-27T10:00:02 INFO server started".to_string(),
    ];
    let mut app = App::new(lines);

    let response = AiFilterResponse {
        text: Some("auth".to_string()),
        min_level: None,
        time_range: None,
    };
    app.apply_ai_filter(&response);

    assert_eq!(app.total_lines(), 2);
}

#[test]
fn test_apply_ai_filter_level_only() {
    let lines = vec![
        "2026-03-27T10:00:00 INFO all good".to_string(),
        "2026-03-27T10:00:01 ERROR something broke".to_string(),
        "2026-03-27T10:00:02 WARN watch out".to_string(),
    ];
    let mut app = App::new(lines);

    let response = AiFilterResponse {
        text: None,
        min_level: Some("ERROR".to_string()),
        time_range: None,
    };
    app.apply_ai_filter(&response);

    assert_eq!(app.total_lines(), 1);
}

#[test]
fn test_apply_ai_filter_empty_response() {
    let lines = vec![
        "2026-03-27T10:00:00 INFO hello".to_string(),
        "2026-03-27T10:00:01 ERROR world".to_string(),
    ];
    let mut app = App::new(lines);

    let response = AiFilterResponse::default();
    app.apply_ai_filter(&response);

    assert_eq!(app.total_lines(), 2);
}
