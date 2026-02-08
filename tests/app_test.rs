use lumolog::app::App;

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
