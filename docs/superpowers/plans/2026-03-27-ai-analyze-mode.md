# AI Analyze Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an analyze mode (`A`) that sends all filtered log lines to the AI with a user question, and displays the analysis response in a scrollable overlay.

**Architecture:** Extends the existing AI infrastructure in `ai.rs` with a new prompt builder for analysis. Adds `Analyze` mode and an overlay response viewer to the app. Uses the same background thread + channel pattern, but with an enum wrapper to distinguish filter vs analyze responses.

**Tech Stack:** Rust, ratatui, existing `ai.rs` + `reqwest` infrastructure

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/ai.rs` | Modify | Add `build_analyze_prompt()` function |
| `src/app.rs` | Modify | Add `Analyze` mode, analyze state fields, overlay scroll methods |
| `src/command.rs` | Modify | Add `EnterAnalyzeMode` action |
| `src/main.rs` | Modify | Change channel type to enum, add Analyze input/overlay handling, response polling |
| `src/ui.rs` | Modify | Render analyze input bar, render analysis overlay panel |
| `tests/ai_test.rs` | Modify | Add tests for `build_analyze_prompt` |

---

### Task 1: Add Analyze Prompt Builder

**Files:**
- Modify: `src/ai.rs`
- Modify: `tests/ai_test.rs`

- [ ] **Step 1: Write tests for build_analyze_prompt**

Add to `tests/ai_test.rs`:

```rust
use lumolog::ai::build_analyze_prompt;

#[test]
fn test_build_analyze_prompt() {
    let lines = vec![
        "2026-03-27T10:00:00 ERROR auth login failed".to_string(),
        "2026-03-27T10:00:01 ERROR auth login failed".to_string(),
    ];
    let (system, user_msg) = build_analyze_prompt("what's wrong?", &lines);
    assert!(system.contains("log analysis expert"));
    assert!(user_msg.contains("2 log lines"));
    assert!(user_msg.contains("auth login failed"));
    assert!(user_msg.contains("what's wrong?"));
}

#[test]
fn test_build_analyze_prompt_empty_lines() {
    let (system, user_msg) = build_analyze_prompt("summarize", &[]);
    assert!(system.contains("log analysis expert"));
    assert!(user_msg.contains("0 log lines"));
    assert!(user_msg.contains("summarize"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test ai_test test_build_analyze`
Expected: FAIL — `build_analyze_prompt` not found

- [ ] **Step 3: Implement build_analyze_prompt**

Add to `src/ai.rs`, after the `build_system_prompt` function:

```rust
/// Build prompts for analyze mode — the AI reads actual log content and answers a question.
/// Returns (system_prompt, user_message).
pub fn build_analyze_prompt(user_question: &str, log_lines: &[String]) -> (String, String) {
    let system = "You are a log analysis expert. The user will show you log lines and ask a question.\n\
        Analyze the logs carefully and provide a clear, concise response. Focus on:\n\
        - Patterns and trends you observe\n\
        - Errors, anomalies, or concerning behavior\n\
        - Correlations between events\n\
        - Potential root causes if errors are present\n\
        - A brief summary of what the logs show\n\n\
        Be specific — reference actual log content, timestamps, and error messages.\n\
        Keep your response concise and actionable."
        .to_string();

    let mut user_msg = format!("Here are {} log lines:\n\n", log_lines.len());
    for line in log_lines {
        user_msg.push_str(line);
        user_msg.push('\n');
    }
    user_msg.push_str(&format!("\nQuestion: {user_question}"));

    (system, user_msg)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test ai_test`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/ai.rs tests/ai_test.rs
git commit -m "feat(ai): add build_analyze_prompt for analyze mode"
```

---

### Task 2: App State — Analyze Mode and Overlay

**Files:**
- Modify: `src/app.rs`
- Modify: `src/command.rs`

- [ ] **Step 1: Add Analyze to AppMode enum**

In `src/app.rs`, add `Analyze` after `Ask`:

```rust
pub enum AppMode {
    Normal,
    Filter,
    ContextMenu,
    Cursor,
    CommandPalette,
    TimeRange,
    Ask,
    Analyze,
}
```

- [ ] **Step 2: Add analyze state fields to App struct**

Add after the `ai_error` field:

```rust
    analyze_input: String,
    analyze_response: Option<String>,
    analyze_scroll: usize,
```

Initialize in `App::new()` after `ai_error: None,`:

```rust
    analyze_input: String::new(),
    analyze_response: None,
    analyze_scroll: 0,
```

- [ ] **Step 3: Add analyze methods to App**

Add after the existing AI methods:

```rust
    // Analyze mode methods

    pub fn enter_analyze_mode(&mut self) {
        if !self.ai_connected {
            return;
        }
        self.analyze_input.clear();
        self.ai_error = None;
        self.mode = AppMode::Analyze;
    }

    pub fn exit_analyze_mode(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn analyze_input(&self) -> &str {
        &self.analyze_input
    }

    pub fn analyze_type(&mut self, c: char) {
        self.analyze_input.push(c);
    }

    pub fn analyze_backspace(&mut self) {
        self.analyze_input.pop();
    }

    pub fn analyze_response(&self) -> Option<&str> {
        self.analyze_response.as_deref()
    }

    pub fn set_analyze_response(&mut self, text: String) {
        self.analyze_response = Some(text);
        self.analyze_scroll = 0;
    }

    pub fn clear_analyze_response(&mut self) {
        self.analyze_response = None;
        self.analyze_scroll = 0;
    }

    pub fn analyze_scroll(&self) -> usize {
        self.analyze_scroll
    }

    pub fn analyze_scroll_down(&mut self, n: usize) {
        self.analyze_scroll = self.analyze_scroll.saturating_add(n);
    }

    pub fn analyze_scroll_up(&mut self, n: usize) {
        self.analyze_scroll = self.analyze_scroll.saturating_sub(n);
    }
```

- [ ] **Step 4: Add EnterAnalyzeMode action**

In `src/command.rs`, add to the `Action` enum:

```rust
    EnterAnalyzeMode,
```

Add a command entry in `commands()`, after the "AI query" entry:

```rust
        Command {
            name: "AI analyze",
            keybinding: Some("A"),
            action: EnterAnalyzeMode,
        },
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check`
Expected: compiles (warnings about unused fields/variants are expected)

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/command.rs
git commit -m "feat(ai): add Analyze mode, state fields, and overlay scroll methods"
```

---

### Task 3: Event Loop — Analyze Input, Background Query, and Overlay Keys

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Change the channel to use an enum for response types**

At the top of `src/main.rs` (after the existing imports), add:

```rust
enum AiResult {
    Filter(Result<String, String>),
    Analyze(Result<String, String>),
}
```

In `run_event_loop`, change the channel type:

```rust
    let (ai_tx, ai_rx) = mpsc::channel::<AiResult>();
```

- [ ] **Step 2: Update the existing Ask mode Enter handler to wrap results**

In the Ask mode `KeyCode::Enter` handler, change the thread spawn to wrap the result:

```rust
                                    std::thread::spawn(move || {
                                        let result = ai::query_ai(&config, &system_prompt, &query);
                                        let _ = tx.send(AiResult::Filter(result));
                                    });
```

- [ ] **Step 3: Add Analyze mode key handling**

Add a new branch **after** the Ask mode branch and **before** the cursor mode branch:

```rust
                    } else if app.mode() == AppMode::Analyze {
                        match key.code {
                            KeyCode::Esc => app.exit_analyze_mode(),
                            KeyCode::Backspace => app.analyze_backspace(),
                            KeyCode::Char(c) => app.analyze_type(c),
                            KeyCode::Enter => {
                                let question = app.analyze_input().to_string();
                                if !question.is_empty()
                                    && let Some(ref config) = ai_config
                                {
                                    let log_lines: Vec<String> = app
                                        .all_filtered_lines_raw()
                                        .lines()
                                        .map(|s| s.to_string())
                                        .collect();

                                    let (system_prompt, user_msg) =
                                        ai::build_analyze_prompt(&question, &log_lines);

                                    app.set_ai_thinking(true);
                                    app.exit_analyze_mode();

                                    let config = config.clone();
                                    let tx = ai_tx.clone();
                                    std::thread::spawn(move || {
                                        let result =
                                            ai::query_ai(&config, &system_prompt, &user_msg);
                                        let _ = tx.send(AiResult::Analyze(result));
                                    });
                                }
                            }
                            _ => {}
                        }
```

- [ ] **Step 4: Add overlay key handling**

Add a branch **before** the CommandPalette branch (at the very top of key handling), since the overlay should capture keys when visible:

```rust
                    if app.analyze_response().is_some() {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => app.clear_analyze_response(),
                            KeyCode::Down | KeyCode::Char('j') => app.analyze_scroll_down(1),
                            KeyCode::Up | KeyCode::Char('k') => app.analyze_scroll_up(1),
                            KeyCode::PageDown | KeyCode::Char(' ') => app.analyze_scroll_down(10),
                            KeyCode::PageUp => app.analyze_scroll_up(10),
                            _ => {}
                        }
                    } else if app.mode() == AppMode::CommandPalette {
```

- [ ] **Step 5: Update AI response polling to handle both types**

Replace the existing AI response polling block with:

```rust
        // Poll for AI query results
        if app.is_ai_thinking()
            && let Ok(result) = ai_rx.try_recv()
        {
            app.set_ai_thinking(false);
            match result {
                AiResult::Filter(res) => match res {
                    Ok(raw_response) => match ai::parse_ai_response(&raw_response) {
                        Ok(filter_response) => {
                            app.set_ai_error(None);
                            app.apply_ai_filter(&filter_response);
                        }
                        Err(e) => {
                            app.set_ai_error(Some(e));
                        }
                    },
                    Err(e) => {
                        app.set_ai_error(Some(e));
                    }
                },
                AiResult::Analyze(res) => match res {
                    Ok(text) => {
                        app.set_ai_error(None);
                        let cleaned = text.trim().to_string();
                        app.set_analyze_response(cleaned);
                    }
                    Err(e) => {
                        app.set_ai_error(Some(e));
                    }
                },
            }
        }
```

- [ ] **Step 6: Add EnterAnalyzeMode to dispatch_action and keybinding**

In `dispatch_action`:

```rust
        EnterAnalyzeMode => app.enter_analyze_mode(),
```

In Normal mode key handling, add before the `'a'` keybinding:

```rust
                            KeyCode::Char('A') if app.is_ai_connected() => {
                                app.enter_analyze_mode();
                            }
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 8: Commit**

```bash
git add src/main.rs
git commit -m "feat(ai): wire up analyze mode input, background query, and overlay keys"
```

---

### Task 4: UI — Analyze Input Bar and Overlay Panel

**Files:**
- Modify: `src/ui.rs`

- [ ] **Step 1: Include Analyze mode in filter_height**

Update both `filter_height` calculations (there are two — main render and command palette render):

```rust
    let filter_height = if app.is_filter_mode() || app.mode() == AppMode::Ask || app.mode() == AppMode::Analyze {
        1
    } else {
        0
    };
```

- [ ] **Step 2: Render analyze input bar**

After the ask bar rendering block, add:

```rust
    // Render analyze bar if in analyze mode
    if app.mode() == AppMode::Analyze {
        let spans = vec![
            Span::styled(
                "analyze: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(app.analyze_input(), Style::default().fg(Color::White)),
        ];
        let analyze_bar = Paragraph::new(Line::from(spans));
        frame.render_widget(analyze_bar, filter_area);
    }
```

- [ ] **Step 3: Render the analysis overlay panel**

At the end of the `render` function (after the command palette overlay rendering), add:

```rust
    // Analysis response overlay
    if let Some(response) = app.analyze_response() {
        let overlay_width = (area.width * 4 / 5).min(area.width.saturating_sub(4));
        let overlay_height = (area.height * 7 / 10).min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(overlay_width)) / 2;
        let y = (area.height.saturating_sub(overlay_height)) / 2;
        let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

        // Inner area for text (accounting for border)
        let inner_width = overlay_width.saturating_sub(2) as usize;

        // Word-wrap the response text into lines
        let wrapped_lines: Vec<Line> = response
            .lines()
            .flat_map(|line| {
                if line.is_empty() {
                    return vec![Line::from("")];
                }
                let mut result = Vec::new();
                let mut current = String::new();
                for word in line.split_whitespace() {
                    if current.is_empty() {
                        current = word.to_string();
                    } else if current.len() + 1 + word.len() <= inner_width {
                        current.push(' ');
                        current.push_str(word);
                    } else {
                        result.push(Line::from(current.clone()));
                        current = word.to_string();
                    }
                }
                if !current.is_empty() {
                    result.push(Line::from(current));
                }
                result
            })
            .collect();

        let scroll = app.analyze_scroll() as u16;

        let overlay = Paragraph::new(wrapped_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" AI Analysis (Esc to close) ")
                    .style(Style::default().fg(Color::White).bg(Color::Black)),
            )
            .scroll((scroll, 0));

        frame.render_widget(Clear, overlay_area);
        frame.render_widget(overlay, overlay_area);
    }
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 6: Run clippy and fmt**

Run: `cargo clippy` then `cargo fmt`

- [ ] **Step 7: Commit**

```bash
git add src/ui.rs
git commit -m "feat(ai): add analyze input bar and scrollable analysis overlay"
```

---

### Task 5: Final Verification

- [ ] **Step 1: Full build**

Run: `cargo build`
Expected: compiles cleanly

- [ ] **Step 2: Full test suite**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 3: Clippy clean**

Run: `cargo clippy`
Expected: no warnings beyond expected dead_code

- [ ] **Step 4: Manual smoke test without AI**

Run: `cargo run -- testdata/sample_json.log`
Expected: app opens normally, no AI indicator, `A` key does nothing

- [ ] **Step 5: Manual smoke test with AI**

Run: `cargo run -- --ai-provider=openai --ai-endpoint=http://127.0.0.1:1234/v1 --ai-model=qwen/qwen3.5-9b testdata/sample_large.log`
Expected:
1. AI indicator visible in status bar
2. Filter with `/` to narrow logs (e.g., type "payment")
3. Press `A`, type "what's going on with these?", press Enter
4. Status bar shows "AI analyzing..."
5. Overlay panel appears with analysis text
6. Scroll with j/k
7. Esc dismisses overlay
