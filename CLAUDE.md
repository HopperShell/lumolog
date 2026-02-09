# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # Debug build
cargo build --release                # Release build
cargo test                           # Run all tests
cargo test --test parser_test        # Run a single test file
cargo test test_detect_json_format   # Run a single test by name
cargo run -- testdata/sample_json.log  # Run with a test log file
cargo clippy                         # Lint
cargo fmt                            # Format
```

Test files live in `tests/` and use the `lumolog` library crate (`src/lib.rs` re-exports all public modules). Test data files are in `testdata/`.

## Architecture

Lumolog is a terminal log file viewer. The data flows through a pipeline:

**Source** (`source.rs`) → **Parser** (`parser.rs`) → **Highlighter** (`highlighter.rs`) → **TUI** (`ui.rs`)

- **Source**: `FileSource` uses `memmap2` for memory-mapped file I/O. `StdinSource` reads piped input. Both produce `Vec<String>`.
- **Parser**: `detect_format()` samples the first 10 lines to classify as `Json`, `Syslog`, or `Plain`. `parse_line()` extracts structured fields (`level`, `timestamp`, `message`) using regex and serde_json.
- **Highlighter**: `highlight_line()` converts a `ParsedLine` into styled ratatui `Line`/`Span` objects. `highlight_line_expanded()` handles JSON pretty-print mode (multi-line output).
- **App** (`app.rs`): Central state — holds parsed lines, scroll position, filter state, mode (`Normal`/`Filter`). Filtering recomputes `filtered_indices` via `filter.rs`.
- **UI** (`ui.rs`): Single `render()` function draws the main view, filter bar, status bar, and help overlay using ratatui widgets.
- **Event loop** (`main.rs`): Synchronous loop using crossterm. Handles key dispatch for Normal mode (vim-like navigation) and Filter mode (text input).

## Key Patterns

- The binary (`main.rs`) has its own `mod` declarations; the library (`lib.rs`) re-exports with `pub mod` for integration tests.
- Lines are parsed once at startup and stored as `Vec<ParsedLine>`. Filtering operates on indices into this vec, not copies.
- JSON log lines try multiple common field names for level (`level`, `severity`, `log.level`) and timestamp (`timestamp`, `time`, `@timestamp`, `ts`).
- Rust edition 2024. Uses `LazyLock` for static regex compilation (no `lazy_static` or `once_cell` crate).
