use lumolog::app::{App, AppMode};
use lumolog::parser::{LogFormat, LogLevel};

#[test]
fn test_scroll_down() {
    let lines: Vec<String> = (0..100).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.scroll_down(1);
    assert_eq!(app.scroll_offset(), 1);
}

#[test]
fn test_scroll_up_clamps_to_zero() {
    let lines: Vec<String> = (0..100).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.scroll_up(5);
    assert_eq!(app.scroll_offset(), 0);
}

#[test]
fn test_scroll_down_clamps_to_max() {
    let lines: Vec<String> = (0..10).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.set_viewport_height(5);
    app.scroll_down(100);
    assert_eq!(app.scroll_offset(), 5);
}

#[test]
fn test_quit() {
    let lines: Vec<String> = vec!["test".into()];
    let mut app = App::new(lines);
    assert!(!app.should_quit());
    app.quit();
    assert!(app.should_quit());
}

// Cursor mode tests

#[test]
fn test_enter_cursor_mode() {
    let lines: Vec<String> = (0..50).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.set_viewport_height(10);
    app.scroll_down(5);
    assert_eq!(app.scroll_offset(), 5);

    app.enter_cursor_mode();
    assert_eq!(app.mode(), AppMode::Cursor);
    assert!(app.is_cursor_mode());
    // Cursor spawns at scroll_offset (top visible line)
    assert_eq!(app.cursor_position(), 5);
}

#[test]
fn test_exit_cursor_mode() {
    let lines: Vec<String> = (0..50).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.enter_cursor_mode();
    assert_eq!(app.mode(), AppMode::Cursor);

    app.exit_cursor_mode();
    assert_eq!(app.mode(), AppMode::Normal);
    assert!(!app.is_cursor_mode());
}

#[test]
fn test_cursor_down() {
    let lines: Vec<String> = (0..50).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.set_viewport_height(10);
    app.enter_cursor_mode();
    assert_eq!(app.cursor_position(), 0);

    app.cursor_down(1);
    assert_eq!(app.cursor_position(), 1);

    app.cursor_down(3);
    assert_eq!(app.cursor_position(), 4);
}

#[test]
fn test_cursor_up() {
    let lines: Vec<String> = (0..50).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.set_viewport_height(10);
    app.scroll_down(10);
    app.enter_cursor_mode();
    assert_eq!(app.cursor_position(), 10);

    app.cursor_up(1);
    assert_eq!(app.cursor_position(), 9);

    app.cursor_up(3);
    assert_eq!(app.cursor_position(), 6);
}

#[test]
fn test_cursor_clamps_at_bounds() {
    let lines: Vec<String> = (0..10).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.set_viewport_height(5);
    app.enter_cursor_mode();

    // Can't go above 0
    app.cursor_up(100);
    assert_eq!(app.cursor_position(), 0);

    // Can't go past last entry
    app.cursor_down(100);
    assert_eq!(app.cursor_position(), 9);
}

#[test]
fn test_cursor_scrolls_viewport_down() {
    let lines: Vec<String> = (0..50).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.set_viewport_height(10);
    app.enter_cursor_mode();
    assert_eq!(app.scroll_offset(), 0);

    // Move cursor past the bottom of the viewport
    app.cursor_down(15);
    assert_eq!(app.cursor_position(), 15);
    // Viewport should have scrolled so cursor is visible
    assert!(app.scroll_offset() > 0);
    assert!(app.scroll_offset() <= app.cursor_position());
}

#[test]
fn test_cursor_scrolls_viewport_up() {
    let lines: Vec<String> = (0..50).map(|i| format!("Line {}", i)).collect();
    let mut app = App::new(lines);
    app.set_viewport_height(10);
    app.scroll_down(20);
    app.enter_cursor_mode();
    assert_eq!(app.cursor_position(), 20);

    // Move cursor above the viewport
    app.cursor_up(25);
    assert_eq!(app.cursor_position(), 0);
    // Viewport should have scrolled up to follow
    assert_eq!(app.scroll_offset(), 0);
}

#[test]
fn test_append_lines_detects_format_when_starting_empty() {
    let mut app = App::new(vec![]);
    assert_eq!(app.format(), LogFormat::Plain);

    app.append_lines(vec![
        r#"{"level":"info","message":"hello","timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
    ]);

    assert_eq!(app.format(), LogFormat::Json);
}

// Level counts and set_min_level tests

#[test]
fn test_level_counts_empty_for_plain_text() {
    let lines: Vec<String> = vec!["hello".into(), "world".into()];
    let app = App::new(lines);
    assert!(app.level_counts().is_empty());
}

#[test]
fn test_level_counts_with_json_logs() {
    let lines: Vec<String> = vec![
        r#"{"level":"info","message":"a","timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
        r#"{"level":"error","message":"b","timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
        r#"{"level":"info","message":"c","timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
        r#"{"level":"warn","message":"d","timestamp":"2024-01-01T00:00:03Z"}"#.to_string(),
    ];
    let app = App::new(lines);
    let counts = app.level_counts();
    // BTreeMap sorts by LogLevel order: Info, Warn, Error
    assert_eq!(counts.len(), 3);
    assert!(counts.contains(&(LogLevel::Info, 2)));
    assert!(counts.contains(&(LogLevel::Warn, 1)));
    assert!(counts.contains(&(LogLevel::Error, 1)));
}

#[test]
fn test_set_min_level_toggles() {
    let lines: Vec<String> = vec![
        r#"{"level":"info","message":"a","timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
        r#"{"level":"error","message":"b","timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
        r#"{"level":"warn","message":"c","timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
    ];
    let mut app = App::new(lines);
    assert_eq!(app.min_level(), None);
    assert_eq!(app.total_lines(), 3);

    // Set to Warn — should show Warn + Error (2 lines)
    app.set_min_level(LogLevel::Warn);
    assert_eq!(app.min_level(), Some(LogLevel::Warn));
    assert_eq!(app.total_lines(), 2);

    // Set to Warn again — should toggle off (clear), show all 3
    app.set_min_level(LogLevel::Warn);
    assert_eq!(app.min_level(), None);
    assert_eq!(app.total_lines(), 3);
}

#[test]
fn test_set_min_level_changes_level() {
    let lines: Vec<String> = vec![
        r#"{"level":"info","message":"a","timestamp":"2024-01-01T00:00:00Z"}"#.to_string(),
        r#"{"level":"error","message":"b","timestamp":"2024-01-01T00:00:01Z"}"#.to_string(),
        r#"{"level":"warn","message":"c","timestamp":"2024-01-01T00:00:02Z"}"#.to_string(),
    ];
    let mut app = App::new(lines);

    // Set to Error — only Error shows
    app.set_min_level(LogLevel::Error);
    assert_eq!(app.min_level(), Some(LogLevel::Error));
    assert_eq!(app.total_lines(), 1);

    // Change to Warn — Warn + Error show
    app.set_min_level(LogLevel::Warn);
    assert_eq!(app.min_level(), Some(LogLevel::Warn));
    assert_eq!(app.total_lines(), 2);
}
