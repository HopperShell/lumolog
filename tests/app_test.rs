use lumolog::app::{App, AppMode};

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
