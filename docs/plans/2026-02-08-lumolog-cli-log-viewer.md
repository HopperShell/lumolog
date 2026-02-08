# Lumolog Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a fast, zero-config terminal log viewer that auto-detects log formats, colorizes output, pretty-prints JSON, and supports interactive filtering and scrolling through large files.

**Architecture:** A pipeline of `Source -> Parser -> Highlighter -> TUI`. The Source reads lines lazily from files or stdin. The Parser auto-detects format (JSON, syslog, plain text) and extracts structured fields. The Highlighter applies color rules based on log level and format. The TUI renders a scrollable, filterable list of styled log lines using ratatui.

**Tech Stack:** Rust, ratatui (TUI framework + crossterm backend), clap (CLI args), serde_json (JSON parsing), memmap2 (memory-mapped file I/O), regex (pattern matching)

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | latest | TUI framework (includes crossterm backend by default) |
| `crossterm` | latest | Terminal event handling (re-exported by ratatui, but needed directly for raw mode) |
| `clap` | latest | CLI argument parsing with derive macros |
| `serde_json` | latest | JSON log line parsing and pretty-printing |
| `memmap2` | latest | Memory-mapped file I/O for large files |
| `regex` | latest | Log format detection and pattern matching |
| `unicode-width` | latest | Correct terminal width calculation for unicode |
| `atty` | latest | Detect if stdin is a TTY or pipe |

---

## Project Structure

```
lumolog/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI arg parsing, orchestration
│   ├── source.rs            # Line reading from files and stdin
│   ├── parser.rs            # Log format detection and structured parsing
│   ├── highlighter.rs       # Color/style rules for log lines
│   ├── app.rs               # Application state and event handling
│   ├── ui.rs                # TUI rendering (ratatui widgets)
│   └── filter.rs            # Interactive filtering logic
├── tests/
│   ├── source_test.rs
│   ├── parser_test.rs
│   ├── highlighter_test.rs
│   └── filter_test.rs
├── testdata/
│   ├── sample_json.log
│   ├── sample_syslog.log
│   └── sample_plain.log
└── docs/
    └── plans/
```

---

## Milestone 1: File Reader + Plain Text Pager (Scrollable `less`-like viewer)

**What you get:** `lumolog <file>` opens a file in a scrollable TUI. Up/down/page-up/page-down to navigate. `q` to quit. No colors yet, just a working pager.

### Task 1.1: Initialize the Rust project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

**Step 1: Create the project with cargo**

Run:
```bash
cd /Users/andrew/Projects/lumolog
cargo init
```
Expected: Creates `Cargo.toml` and `src/main.rs`

**Step 2: Add dependencies**

Run:
```bash
cargo add ratatui crossterm clap --features clap/derive
cargo add serde_json memmap2 regex unicode-width
```

**Step 3: Verify it compiles**

Run:
```bash
cargo build
```
Expected: BUILD SUCCESS

**Step 4: Commit**

```bash
git init
git add -A
git commit -m "chore: initialize lumolog project with dependencies"
```

---

### Task 1.2: CLI argument parsing

**Files:**
- Create: `src/main.rs` (replace scaffold)

**Step 1: Write the test**

Create `tests/cli_test.rs`:
```rust
use std::process::Command;

#[test]
fn test_missing_file_shows_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "nonexistent_file.log"])
        .output()
        .expect("failed to execute");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("nonexistent_file.log")
            || output.status.code() != Some(0),
        "Should error on missing file"
    );
}

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lumolog") || stdout.contains("USAGE") || stdout.contains("Usage"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test cli_test`
Expected: FAIL (main.rs is just hello world)

**Step 3: Implement CLI parsing in main.rs**

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "lumolog", version, about = "A terminal log viewer that makes logs readable")]
struct Cli {
    /// Log file to view. Omit to read from stdin.
    file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.file {
        Some(path) => {
            if !path.exists() {
                eprintln!("Error: file not found: {}", path.display());
                std::process::exit(1);
            }
            println!("Would open: {}", path.display());
        }
        None => {
            println!("Would read from stdin");
        }
    }

    Ok(())
}
```

Also add `anyhow` dependency:
```bash
cargo add anyhow
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test cli_test`
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add CLI argument parsing with clap"
```

---

### Task 1.3: File source — lazy line reading

**Files:**
- Create: `src/source.rs`
- Create: `tests/source_test.rs`
- Create: `testdata/sample_plain.log`

**Step 1: Create test data**

Create `testdata/sample_plain.log`:
```
2024-01-15 08:30:01 INFO  Application starting up
2024-01-15 08:30:02 DEBUG Loading configuration from /etc/app/config.yaml
2024-01-15 08:30:02 INFO  Connected to database at localhost:5432
2024-01-15 08:30:03 WARN  Cache miss rate above threshold: 45%
2024-01-15 08:30:05 ERROR Failed to connect to redis: Connection refused
2024-01-15 08:30:05 INFO  Falling back to in-memory cache
2024-01-15 08:30:06 DEBUG Request received: GET /api/users
2024-01-15 08:30:06 INFO  Response sent: 200 OK (23ms)
2024-01-15 08:30:07 WARN  Slow query detected: SELECT * FROM users (850ms)
2024-01-15 08:30:10 ERROR Unhandled exception in worker thread #3
```

**Step 2: Write the test**

Create `tests/source_test.rs`:
```rust
use lumolog::source::FileSource;

#[test]
fn test_file_source_reads_lines() {
    let source = FileSource::open("testdata/sample_plain.log").unwrap();
    let lines = source.lines();
    assert_eq!(lines.len(), 10);
    assert!(lines[0].contains("Application starting up"));
    assert!(lines[9].contains("Unhandled exception"));
}

#[test]
fn test_file_source_line_count() {
    let source = FileSource::open("testdata/sample_plain.log").unwrap();
    assert_eq!(source.line_count(), 10);
}

#[test]
fn test_file_source_missing_file() {
    let result = FileSource::open("nonexistent.log");
    assert!(result.is_err());
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test --test source_test`
Expected: FAIL (module doesn't exist)

**Step 4: Implement FileSource**

Create `src/source.rs`:
```rust
use std::fs;
use std::path::Path;

pub struct FileSource {
    lines: Vec<String>,
}

impl FileSource {
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        Ok(Self { lines })
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}
```

Add `src/lib.rs` to expose modules:
```rust
pub mod source;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test source_test`
Expected: PASS

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: add FileSource for reading log files"
```

---

### Task 1.4: Basic TUI pager — scrollable view

**Files:**
- Create: `src/app.rs`
- Create: `src/ui.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

**Step 1: Write the test for App state**

Create `tests/app_test.rs`:
```rust
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
    // Simulate a viewport of 5 lines
    app.set_viewport_height(5);
    app.scroll_down(100);
    // Should clamp: max offset = 10 - 5 = 5
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test app_test`
Expected: FAIL

**Step 3: Implement App**

Create `src/app.rs`:
```rust
pub struct App {
    lines: Vec<String>,
    scroll_offset: usize,
    viewport_height: usize,
    quit: bool,
}

impl App {
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            lines,
            scroll_offset: 0,
            viewport_height: 24, // default, updated on render
            quit: false,
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
        self.clamp_scroll();
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
        self.clamp_scroll();
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_height.saturating_sub(2));
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_height.saturating_sub(2));
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        if self.lines.len() > self.viewport_height {
            self.scroll_offset = self.lines.len() - self.viewport_height;
        }
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    fn clamp_scroll(&mut self) {
        let max = self.lines.len().saturating_sub(self.viewport_height);
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    pub fn visible_lines(&self) -> &[String] {
        let start = self.scroll_offset;
        let end = (start + self.viewport_height).min(self.lines.len());
        &self.lines[start..end]
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test app_test`
Expected: PASS

**Step 5: Implement UI rendering**

Create `src/ui.rs`:
```rust
use ratatui::Frame;
use ratatui::layout::{Layout, Constraint};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let [main_area, status_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
    ]).areas(area);

    // Update viewport height (subtract 2 for border)
    let content_height = main_area.height.saturating_sub(2) as usize;
    app.set_viewport_height(content_height);

    // Render log lines
    let visible: Vec<Line> = app
        .visible_lines()
        .iter()
        .map(|line| Line::raw(line.as_str()))
        .collect();

    let log_view = Paragraph::new(visible)
        .block(Block::default().borders(Borders::ALL).title("lumolog"));

    frame.render_widget(log_view, main_area);

    // Status bar
    let total = app.lines().len();
    let offset = app.scroll_offset();
    let pct = if total == 0 {
        100
    } else {
        ((offset + content_height).min(total) * 100) / total
    };
    let status_text = format!(
        " Line {}-{} of {} ({}%) | q:quit  j/k:scroll  PgUp/PgDn  g/G:top/bottom",
        offset + 1,
        (offset + content_height).min(total),
        total,
        pct
    );
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Black).bg(Color::White));

    frame.render_widget(status, status_area);
}
```

**Step 6: Wire up main.rs with the TUI event loop**

Replace `src/main.rs`:
```rust
mod app;
mod source;
mod ui;

use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use source::FileSource;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "lumolog", version, about = "A terminal log viewer that makes logs readable")]
struct Cli {
    /// Log file to view. Omit to read from stdin.
    file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let lines = match &cli.file {
        Some(path) => {
            if !path.exists() {
                eprintln!("Error: file not found: {}", path.display());
                std::process::exit(1);
            }
            let source = FileSource::open(path)?;
            source.lines().to_vec()
        }
        None => {
            eprintln!("stdin support coming soon. Please provide a file.");
            std::process::exit(1);
        }
    };

    let mut terminal = ratatui::init();
    let mut app = App::new(lines);

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.quit(),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
                    KeyCode::PageDown | KeyCode::Char(' ') => app.page_down(),
                    KeyCode::PageUp => app.page_up(),
                    KeyCode::Char('g') => app.scroll_to_top(),
                    KeyCode::Char('G') => app.scroll_to_bottom(),
                    _ => {}
                }
            }
        }

        if app.should_quit() {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}
```

Update `src/lib.rs`:
```rust
pub mod app;
pub mod source;
pub mod ui;
```

**Step 7: Manual test**

Run: `cargo run -- testdata/sample_plain.log`
Expected: See log lines in a bordered TUI. Scroll with j/k, page with PgUp/PgDn, quit with q.

**Step 8: Commit**

```bash
git add -A
git commit -m "feat: basic TUI pager with scrollable log view"
```

---

## Milestone 2: Log Format Detection + Syntax Highlighting

**What you get:** `lumolog <file>` auto-detects whether the file contains JSON logs, syslog, or plain text and applies appropriate colorization. Errors are red, warnings yellow, timestamps dimmed, JSON is pretty-printed inline.

### Task 2.1: Log format detection (parser)

**Files:**
- Create: `src/parser.rs`
- Create: `tests/parser_test.rs`
- Create: `testdata/sample_json.log`
- Create: `testdata/sample_syslog.log`

**Step 1: Create test data**

Create `testdata/sample_json.log`:
```
{"timestamp":"2024-01-15T08:30:01Z","level":"info","message":"Application starting up","service":"api"}
{"timestamp":"2024-01-15T08:30:02Z","level":"debug","message":"Loading configuration","path":"/etc/app/config.yaml"}
{"timestamp":"2024-01-15T08:30:03Z","level":"warn","message":"Cache miss rate above threshold","rate":0.45}
{"timestamp":"2024-01-15T08:30:05Z","level":"error","message":"Failed to connect to redis","error":"Connection refused","host":"localhost:6379"}
{"timestamp":"2024-01-15T08:30:06Z","level":"info","message":"Request received","method":"GET","path":"/api/users","duration_ms":23}
```

Create `testdata/sample_syslog.log`:
```
Jan 15 08:30:01 myhost sshd[1234]: Accepted publickey for user from 192.168.1.100 port 52413
Jan 15 08:30:02 myhost kernel: [  123.456789] usb 1-1: new high-speed USB device number 2
Jan 15 08:30:03 myhost systemd[1]: Started Application Service.
Jan 15 08:30:05 myhost app[5678]: ERROR: Database connection timeout after 30s
Jan 15 08:30:06 myhost app[5678]: WARNING: Retrying connection (attempt 2/5)
```

**Step 2: Write the tests**

Create `tests/parser_test.rs`:
```rust
use lumolog::parser::{detect_format, LogFormat, parse_line, ParsedLine, LogLevel};

#[test]
fn test_detect_json_format() {
    let lines = vec![
        r#"{"timestamp":"2024-01-15T08:30:01Z","level":"info","message":"test"}"#.to_string(),
        r#"{"timestamp":"2024-01-15T08:30:02Z","level":"debug","message":"test2"}"#.to_string(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Json);
}

#[test]
fn test_detect_syslog_format() {
    let lines = vec![
        "Jan 15 08:30:01 myhost sshd[1234]: Accepted publickey".to_string(),
        "Jan 15 08:30:02 myhost kernel: something".to_string(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Syslog);
}

#[test]
fn test_detect_plain_format() {
    let lines = vec![
        "2024-01-15 08:30:01 INFO  Application starting up".to_string(),
        "2024-01-15 08:30:02 DEBUG Loading configuration".to_string(),
    ];
    assert_eq!(detect_format(&lines), LogFormat::Plain);
}

#[test]
fn test_parse_json_line() {
    let line = r#"{"timestamp":"2024-01-15T08:30:05Z","level":"error","message":"Failed to connect"}"#;
    let parsed = parse_line(line, LogFormat::Json);
    assert_eq!(parsed.level, Some(LogLevel::Error));
    assert!(parsed.timestamp.is_some());
    assert!(parsed.message.contains("Failed to connect"));
}

#[test]
fn test_parse_plain_line_error() {
    let line = "2024-01-15 08:30:05 ERROR Failed to connect to redis";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Error));
}

#[test]
fn test_parse_plain_line_warn() {
    let line = "2024-01-15 08:30:03 WARN  Cache miss rate above threshold";
    let parsed = parse_line(line, LogFormat::Plain);
    assert_eq!(parsed.level, Some(LogLevel::Warn));
}

#[test]
fn test_parse_syslog_line() {
    let line = "Jan 15 08:30:01 myhost sshd[1234]: Accepted publickey for user";
    let parsed = parse_line(line, LogFormat::Syslog);
    assert!(parsed.timestamp.is_some());
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test --test parser_test`
Expected: FAIL

**Step 4: Implement the parser**

Create `src/parser.rs`:
```rust
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Syslog,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[derive(Debug, Clone)]
pub struct ParsedLine {
    pub raw: String,
    pub level: Option<LogLevel>,
    pub timestamp: Option<String>,
    pub message: String,
    pub format: LogFormat,
    /// For JSON lines: the pretty-printed version
    pub pretty_json: Option<String>,
}

static SYSLOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+(.+)$").unwrap()
});

static PLAIN_TIMESTAMP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}[^\s]*)").unwrap()
});

static LEVEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(TRACE|DEBUG|INFO|WARN(?:ING)?|ERROR|FATAL|CRITICAL|SEVERE)\b").unwrap()
});

/// Detect log format by sampling the first few lines.
pub fn detect_format(lines: &[String]) -> LogFormat {
    let sample: Vec<&str> = lines.iter().take(10).map(|s| s.as_str()).collect();
    if sample.is_empty() {
        return LogFormat::Plain;
    }

    // Check JSON: try to parse first few lines as JSON objects
    let json_count = sample
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('{') && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        })
        .count();
    if json_count > sample.len() / 2 {
        return LogFormat::Json;
    }

    // Check syslog: matches "Mon DD HH:MM:SS hostname"
    let syslog_count = sample.iter().filter(|line| SYSLOG_RE.is_match(line)).count();
    if syslog_count > sample.len() / 2 {
        return LogFormat::Syslog;
    }

    LogFormat::Plain
}

pub fn parse_line(raw: &str, format: LogFormat) -> ParsedLine {
    match format {
        LogFormat::Json => parse_json_line(raw),
        LogFormat::Syslog => parse_syslog_line(raw),
        LogFormat::Plain => parse_plain_line(raw),
    }
}

fn parse_json_line(raw: &str) -> ParsedLine {
    let trimmed = raw.trim();
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => {
            let level = value
                .get("level")
                .or_else(|| value.get("severity"))
                .or_else(|| value.get("log.level"))
                .and_then(|v| v.as_str())
                .and_then(parse_level_str);

            let timestamp = value
                .get("timestamp")
                .or_else(|| value.get("time"))
                .or_else(|| value.get("@timestamp"))
                .or_else(|| value.get("ts"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let message = value
                .get("message")
                .or_else(|| value.get("msg"))
                .and_then(|v| v.as_str())
                .unwrap_or(trimmed)
                .to_string();

            let pretty = serde_json::to_string_pretty(&value).ok();

            ParsedLine {
                raw: raw.to_string(),
                level,
                timestamp,
                message,
                format: LogFormat::Json,
                pretty_json: pretty,
            }
        }
        Err(_) => ParsedLine {
            raw: raw.to_string(),
            level: None,
            timestamp: None,
            message: raw.to_string(),
            format: LogFormat::Json,
            pretty_json: None,
        },
    }
}

fn parse_syslog_line(raw: &str) -> ParsedLine {
    let (timestamp, message) = if let Some(caps) = SYSLOG_RE.captures(raw) {
        (
            Some(caps[1].to_string()),
            caps[3].to_string(),
        )
    } else {
        (None, raw.to_string())
    };

    let level = LEVEL_RE
        .find(raw)
        .and_then(|m| parse_level_str(m.as_str()));

    ParsedLine {
        raw: raw.to_string(),
        level,
        timestamp,
        message,
        format: LogFormat::Syslog,
        pretty_json: None,
    }
}

fn parse_plain_line(raw: &str) -> ParsedLine {
    let timestamp = PLAIN_TIMESTAMP_RE
        .find(raw)
        .map(|m| m.as_str().to_string());

    let level = LEVEL_RE
        .find(raw)
        .and_then(|m| parse_level_str(m.as_str()));

    ParsedLine {
        raw: raw.to_string(),
        level,
        timestamp,
        message: raw.to_string(),
        format: LogFormat::Plain,
        pretty_json: None,
    }
}

fn parse_level_str(s: &str) -> Option<LogLevel> {
    match s.to_uppercase().as_str() {
        "TRACE" => Some(LogLevel::Trace),
        "DEBUG" => Some(LogLevel::Debug),
        "INFO" => Some(LogLevel::Info),
        "WARN" | "WARNING" => Some(LogLevel::Warn),
        "ERROR" | "SEVERE" => Some(LogLevel::Error),
        "FATAL" | "CRITICAL" => Some(LogLevel::Fatal),
        _ => None,
    }
}
```

Update `src/lib.rs`:
```rust
pub mod app;
pub mod parser;
pub mod source;
pub mod ui;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test parser_test`
Expected: PASS

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: add log format detection and line parsing"
```

---

### Task 2.2: Syntax highlighting

**Files:**
- Create: `src/highlighter.rs`
- Create: `tests/highlighter_test.rs`

**Step 1: Write the tests**

Create `tests/highlighter_test.rs`:
```rust
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
    // The line should contain at least one span with red foreground
    let has_red = styled.spans.iter().any(|span| span.style.fg == Some(Color::Red));
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
    let has_yellow = styled.spans.iter().any(|span| span.style.fg == Some(Color::Yellow));
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
    // Should not be red or yellow
    let has_red = styled.spans.iter().any(|span| span.style.fg == Some(Color::Red));
    let has_yellow = styled.spans.iter().any(|span| span.style.fg == Some(Color::Yellow));
    assert!(!has_red && !has_yellow, "Info lines should not be red or yellow");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test highlighter_test`
Expected: FAIL

**Step 3: Implement the highlighter**

Create `src/highlighter.rs`:
```rust
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::parser::{LogFormat, LogLevel, ParsedLine};

pub fn highlight_line<'a>(parsed: &'a ParsedLine) -> Line<'a> {
    match parsed.format {
        LogFormat::Json => highlight_json_line(parsed),
        LogFormat::Syslog => highlight_syslog_line(parsed),
        LogFormat::Plain => highlight_plain_line(parsed),
    }
}

fn level_style(level: Option<LogLevel>) -> Style {
    match level {
        Some(LogLevel::Fatal) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        Some(LogLevel::Error) => Style::default().fg(Color::Red),
        Some(LogLevel::Warn) => Style::default().fg(Color::Yellow),
        Some(LogLevel::Info) => Style::default().fg(Color::Green),
        Some(LogLevel::Debug) => Style::default().fg(Color::DarkGray),
        Some(LogLevel::Trace) => Style::default().fg(Color::DarkGray),
        None => Style::default(),
    }
}

fn timestamp_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn highlight_plain_line<'a>(parsed: &'a ParsedLine) -> Line<'a> {
    let style = level_style(parsed.level);

    // For plain lines, apply the level color to the whole line
    // but dim the timestamp portion
    if let Some(ref ts) = parsed.timestamp {
        let ts_end = parsed.raw.find(ts.as_str()).unwrap_or(0) + ts.len();
        let (ts_part, rest) = parsed.raw.split_at(ts_end);
        Line::from(vec![
            Span::styled(ts_part.to_string(), timestamp_style()),
            Span::styled(rest.to_string(), style),
        ])
    } else {
        Line::from(Span::styled(parsed.raw.as_str(), style))
    }
}

fn highlight_json_line<'a>(parsed: &'a ParsedLine) -> Line<'a> {
    let style = level_style(parsed.level);

    // Show a compact formatted version: [LEVEL] timestamp message
    let level_str = match parsed.level {
        Some(LogLevel::Fatal) => "FTL",
        Some(LogLevel::Error) => "ERR",
        Some(LogLevel::Warn) => "WRN",
        Some(LogLevel::Info) => "INF",
        Some(LogLevel::Debug) => "DBG",
        Some(LogLevel::Trace) => "TRC",
        None => "???",
    };

    let mut spans = Vec::new();

    // Level badge
    spans.push(Span::styled(
        format!("[{}] ", level_str),
        style.add_modifier(Modifier::BOLD),
    ));

    // Timestamp
    if let Some(ref ts) = parsed.timestamp {
        spans.push(Span::styled(format!("{} ", ts), timestamp_style()));
    }

    // Message
    spans.push(Span::styled(parsed.message.clone(), style));

    Line::from(spans)
}

fn highlight_syslog_line<'a>(parsed: &'a ParsedLine) -> Line<'a> {
    let style = level_style(parsed.level);

    if let Some(ref ts) = parsed.timestamp {
        let ts_end = parsed.raw.find(ts.as_str()).unwrap_or(0) + ts.len();
        let (ts_part, rest) = parsed.raw.split_at(ts_end);
        Line::from(vec![
            Span::styled(ts_part.to_string(), timestamp_style()),
            Span::styled(rest.to_string(), style),
        ])
    } else {
        Line::from(Span::styled(parsed.raw.as_str(), style))
    }
}
```

Update `src/lib.rs`:
```rust
pub mod app;
pub mod highlighter;
pub mod parser;
pub mod source;
pub mod ui;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test highlighter_test`
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add syntax highlighting for log lines"
```

---

### Task 2.3: Integrate parser + highlighter into the TUI

**Files:**
- Modify: `src/app.rs`
- Modify: `src/ui.rs`
- Modify: `src/main.rs`

**Step 1: Update App to hold parsed lines**

Modify `src/app.rs` to store `ParsedLine` values and the detected `LogFormat`:

- Add a `parsed_lines: Vec<ParsedLine>` field
- Add a `format: LogFormat` field
- In `App::new()`, call `detect_format()` on the raw lines, then `parse_line()` each line
- Update `visible_lines()` to return `&[ParsedLine]`

The constructor becomes:
```rust
pub fn new(lines: Vec<String>) -> Self {
    let format = detect_format(&lines);
    let parsed_lines: Vec<ParsedLine> = lines
        .iter()
        .map(|line| parse_line(line, format))
        .collect();
    Self {
        lines,
        parsed_lines,
        format,
        scroll_offset: 0,
        viewport_height: 24,
        quit: false,
    }
}
```

Add a method:
```rust
pub fn visible_parsed_lines(&self) -> &[ParsedLine] {
    let start = self.scroll_offset;
    let end = (start + self.viewport_height).min(self.parsed_lines.len());
    &self.parsed_lines[start..end]
}

pub fn format(&self) -> LogFormat {
    self.format
}
```

**Step 2: Update UI to use highlighted lines**

Modify `src/ui.rs` to call `highlight_line()` on each visible parsed line instead of using raw text:

```rust
let visible: Vec<Line> = app
    .visible_parsed_lines()
    .iter()
    .map(|parsed| highlight_line(parsed))
    .collect();
```

Add the `format` to the title:
```rust
let format_label = match app.format() {
    LogFormat::Json => "JSON",
    LogFormat::Syslog => "Syslog",
    LogFormat::Plain => "Plain",
};
// ...
.title(format!("lumolog [{}]", format_label))
```

**Step 3: Manual test with all three log formats**

Run:
```bash
cargo run -- testdata/sample_plain.log
cargo run -- testdata/sample_json.log
cargo run -- testdata/sample_syslog.log
```
Expected: Each file shows properly colorized output with the format detected in the title bar.

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: integrate format detection and highlighting into TUI"
```

---

## Milestone 3: Interactive Filtering + Stdin Support

**What you get:** Press `/` to enter filter mode, type a pattern, and the view filters to matching lines in real time. Piped input works: `cat app.log | lumolog`.

### Task 3.1: Interactive filtering

**Files:**
- Create: `src/filter.rs`
- Create: `tests/filter_test.rs`
- Modify: `src/app.rs`
- Modify: `src/ui.rs`
- Modify: `src/main.rs`

**Step 1: Write the tests**

Create `tests/filter_test.rs`:
```rust
use lumolog::filter::filter_lines;
use lumolog::parser::{LogFormat, ParsedLine, LogLevel};

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
    let lines = vec![
        make_line("line one", None),
        make_line("line two", None),
    ];
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test filter_test`
Expected: FAIL

**Step 3: Implement filter**

Create `src/filter.rs`:
```rust
use crate::parser::ParsedLine;

/// Returns indices of lines matching the pattern (case-insensitive substring match).
pub fn filter_lines(lines: &[ParsedLine], pattern: &str) -> Vec<usize> {
    if pattern.is_empty() {
        return (0..lines.len()).collect();
    }

    let pattern_lower = pattern.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.raw.to_lowercase().contains(&pattern_lower))
        .map(|(i, _)| i)
        .collect()
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test filter_test`
Expected: PASS

**Step 5: Integrate filter into App state**

Add to `src/app.rs`:
- `filter_pattern: String` field
- `filtered_indices: Vec<usize>` field (indices into `parsed_lines`)
- `mode: AppMode` enum with `Normal` and `Filter` variants
- When filter pattern changes, recompute `filtered_indices` using `filter_lines()`
- `visible_parsed_lines()` now uses `filtered_indices` to select which lines to show

Key new methods:
```rust
pub fn enter_filter_mode(&mut self) {
    self.mode = AppMode::Filter;
}

pub fn exit_filter_mode(&mut self) {
    self.mode = AppMode::Normal;
}

pub fn clear_filter(&mut self) {
    self.filter_pattern.clear();
    self.recompute_filter();
    self.mode = AppMode::Normal;
}

pub fn filter_input(&mut self, c: char) {
    self.filter_pattern.push(c);
    self.recompute_filter();
}

pub fn filter_backspace(&mut self) {
    self.filter_pattern.pop();
    self.recompute_filter();
}

fn recompute_filter(&mut self) {
    self.filtered_indices = filter_lines(&self.parsed_lines, &self.filter_pattern);
    self.scroll_offset = 0;
}
```

**Step 6: Update UI to show filter bar and filtered view**

In `src/ui.rs`, when `app.mode() == AppMode::Filter`, add a filter input bar at the bottom:
```rust
let [main_area, filter_area, status_area] = Layout::vertical([
    Constraint::Fill(1),
    Constraint::Length(if app.is_filter_mode() { 1 } else { 0 }),
    Constraint::Length(1),
]).areas(area);

if app.is_filter_mode() {
    let filter_text = format!("/{}", app.filter_pattern());
    let filter_bar = Paragraph::new(filter_text)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(filter_bar, filter_area);
}
```

**Step 7: Update key handling in main.rs**

In `Normal` mode:
- `/` enters filter mode
- All existing keys still work

In `Filter` mode:
- Typing characters appends to the filter
- `Backspace` removes last char
- `Enter` or `Esc` exits filter mode (keeps filter active)
- `Esc` when pattern is empty clears filter and returns to Normal

**Step 8: Manual test**

Run: `cargo run -- testdata/sample_plain.log`
Press `/`, type `error` — should see only error lines. Press `Esc` to return.

**Step 9: Commit**

```bash
git add -A
git commit -m "feat: add interactive filtering with / command"
```

---

### Task 3.2: Stdin pipe support

**Files:**
- Modify: `src/source.rs`
- Modify: `src/main.rs`

**Step 1: Write the test**

Add to `tests/source_test.rs`:
```rust
use lumolog::source::StdinSource;
use std::io::Cursor;

#[test]
fn test_stdin_source_reads_lines() {
    let input = "line 1\nline 2\nline 3\n";
    let cursor = Cursor::new(input);
    let source = StdinSource::from_reader(cursor);
    let lines = source.lines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line 1");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test source_test`
Expected: FAIL

**Step 3: Implement StdinSource**

Add to `src/source.rs`:
```rust
use std::io::{self, BufRead, Read};

pub struct StdinSource {
    lines: Vec<String>,
}

impl StdinSource {
    /// Read all available input from stdin (for non-streaming use).
    pub fn read_all() -> anyhow::Result<Self> {
        let stdin = io::stdin();
        let lines: Vec<String> = stdin.lock().lines().collect::<Result<_, _>>()?;
        Ok(Self { lines })
    }

    /// For testing: read from any reader.
    pub fn from_reader<R: Read>(reader: R) -> Self {
        let reader = io::BufReader::new(reader);
        let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        Self { lines }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}
```

**Step 4: Update main.rs to detect stdin**

```rust
use std::io::IsTerminal;

// In main():
let lines = match &cli.file {
    Some(path) => {
        if !path.exists() {
            eprintln!("Error: file not found: {}", path.display());
            std::process::exit(1);
        }
        FileSource::open(path)?.lines().to_vec()
    }
    None => {
        if std::io::stdin().is_terminal() {
            eprintln!("Usage: lumolog <file> or pipe input via stdin");
            eprintln!("Example: cat app.log | lumolog");
            std::process::exit(1);
        }
        StdinSource::read_all()?.lines().to_vec()
    }
};
```

Note: remove the `atty` dependency since `std::io::IsTerminal` is stable in recent Rust.

**Step 5: Run tests to verify they pass**

Run: `cargo test --test source_test`
Expected: PASS

**Step 6: Manual test**

Run: `cat testdata/sample_json.log | cargo run`
Expected: JSON lines displayed with highlighting in the TUI.

**Step 7: Commit**

```bash
git add -A
git commit -m "feat: add stdin pipe support"
```

---

## Milestone 4: JSON Pretty-Print Toggle + Large File Handling + Polish

**What you get:** Press `p` to toggle pretty-printed JSON. Large files (100MB+) load fast using memory-mapped I/O. Status bar shows useful context.

### Task 4.1: JSON pretty-print toggle

**Files:**
- Modify: `src/app.rs`
- Modify: `src/ui.rs`
- Modify: `src/highlighter.rs`

**Step 1: Add pretty-print state to App**

Add `json_pretty: bool` field to `App`, default `false`. Add `toggle_pretty()` method.

**Step 2: Update highlighter for pretty mode**

In `highlight_json_line()`, when pretty mode is enabled, return multiple `Line`s instead of one. This means `highlight_line` needs to return `Vec<Line>` (or the UI handles expansion).

Better approach: add a method `highlight_line_expanded()` that returns `Vec<Line>` — for JSON in pretty mode it returns multiple lines (indented JSON), for everything else it returns a single line.

**Step 3: Update UI to handle expanded lines**

In `ui.rs`, for JSON pretty mode, iterate over expanded lines instead of single lines.

**Step 4: Update key handling**

In `main.rs`, `KeyCode::Char('p')` toggles `app.toggle_pretty()`.

**Step 5: Manual test**

Run: `cargo run -- testdata/sample_json.log`
Press `p` — JSON lines should expand to pretty-printed, indented JSON. Press `p` again to collapse.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: add JSON pretty-print toggle with 'p' key"
```

---

### Task 4.2: Memory-mapped large file reading

**Files:**
- Modify: `src/source.rs`
- Add: `tests/source_test.rs` (additional test)

**Step 1: Write the test**

Add to `tests/source_test.rs`:
```rust
use std::io::Write;
use tempfile::NamedTempFile;
use lumolog::source::FileSource;

#[test]
fn test_large_file_line_count() {
    let mut file = NamedTempFile::new().unwrap();
    for i in 0..10_000 {
        writeln!(file, "2024-01-15 INFO Line number {}", i).unwrap();
    }
    let source = FileSource::open(file.path()).unwrap();
    assert_eq!(source.line_count(), 10_000);
}
```

Add `tempfile` as a dev dependency:
```bash
cargo add tempfile --dev
```

**Step 2: Replace naive read_to_string with memmap2**

Update `FileSource::open()`:
```rust
use memmap2::Mmap;
use std::fs::File;

pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;

    if metadata.len() == 0 {
        return Ok(Self { lines: Vec::new() });
    }

    let mmap = unsafe { Mmap::map(&file)? };
    let content = std::str::from_utf8(&mmap)?;
    let lines: Vec<String> = content.lines().map(String::from).collect();
    Ok(Self { lines })
}
```

This gives us fast I/O for large files via OS page caching, without loading the entire file into heap memory upfront.

**Step 3: Run tests**

Run: `cargo test --test source_test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: use memory-mapped I/O for faster large file loading"
```

---

### Task 4.3: Polish — line numbers, help overlay, improved status bar

**Files:**
- Modify: `src/ui.rs`
- Modify: `src/app.rs`
- Modify: `src/main.rs`

**Step 1: Add line numbers**

In `ui.rs`, prefix each visible line with its line number (right-aligned, dimmed):
```rust
let line_num_width = format!("{}", app.total_lines()).len();
let prefix = format!("{:>width$} ", line_idx + 1, width = line_num_width);
```

Prepend a `Span::styled(prefix, Style::default().fg(Color::DarkGray))` to each line.

**Step 2: Add `?` key for help overlay**

In `app.rs`, add `show_help: bool` field. In `main.rs`, `KeyCode::Char('?')` toggles it. In `ui.rs`, when `show_help` is true, render a centered overlay block listing keybindings:

```
 q / Esc    Quit
 j / k      Scroll up/down
 PgUp/PgDn  Page up/down
 g / G      Top / Bottom
 /          Filter
 p          Pretty-print JSON
 ?          Toggle this help
```

**Step 3: Improve status bar**

Show: filename (or "stdin"), detected format, line count, filter status if active.

Example: `sample.log | JSON | 1,234 lines | Filter: "error" (23 matches) | 45%`

**Step 4: Manual test**

Run: `cargo run -- testdata/sample_json.log`
Verify: line numbers visible, `?` shows help, status bar is informative.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: add line numbers, help overlay, improved status bar"
```

---

## Stretch Goals (Post-MVP)

These are not part of the implementation plan but worth tracking for future work:

1. **Tail mode (`-f` / `--follow`)** — Watch a file for new lines and auto-scroll to bottom when new lines appear. For stdin streams (`docker logs -f | lumolog`), buffer incoming lines and append to the view.

2. **Regex filter** — Upgrade the filter from substring to full regex. Toggle with a keybind or prefix the pattern with `r/`.

3. **Multiple files** — `lumolog file1.log file2.log` with tabs or split view.

4. **Bookmarking** — Press `m` to bookmark a line, `n`/`N` to jump between bookmarks.

5. **Copy to clipboard** — Press `y` to yank the current line or visible selection to clipboard.

6. **Config file** — `~/.config/lumolog/config.toml` for custom color themes, keybindings, default settings.

7. **Log level filtering** — Quick keys to show only errors (`e`), warnings and above (`w`), etc.

8. **Search highlighting** — When filtering, highlight the matching portion of each line in a contrasting color.

9. **Time range filtering** — Filter to lines within a time window (e.g., "last 5 minutes").

10. **Homebrew/cargo install distribution** — Publish to crates.io, create a Homebrew formula.

---

## Key Design Decisions

**Why ratatui over cursive/tui-rs?** Ratatui is the actively maintained fork of tui-rs with the largest community. It's the de facto standard for Rust TUI apps.

**Why memmap2 over streaming?** For file-based reading, memory mapping lets the OS handle paging efficiently. We still collect lines into a `Vec<String>` for random access (scrolling requires jumping to arbitrary positions). For truly massive files (10GB+), a future optimization would be a line-offset index that avoids collecting all lines upfront.

**Why not async?** The MVP doesn't need async. The event loop polls for input and renders synchronously. Stdin reading blocks until EOF, which is fine for pipes. Async would be needed for `--follow` mode (stretch goal) and could be added later with tokio.

**Why substring filter over regex?** Simpler to implement, faster, and covers 90% of use cases. Regex can be added as a stretch goal. Case-insensitive substring matching is what most users want when they type `/error`.

**JSON rendering strategy:** Compact by default (one line per log entry showing `[LEVEL] timestamp message`), with `p` to toggle full pretty-printed JSON. This keeps the view dense by default while allowing drill-down.
