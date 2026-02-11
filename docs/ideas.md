# Lumolog Feature Ideas

Core differentiator vs tailspin: tailspin makes text pretty. Lumolog *understands* your logs.

## High Priority

- **Command palette** — press `:` (or `Ctrl+K`) to open a fuzzy-searchable list of every available action, VS Code style. Each entry shows the action name and its keybinding (if any). Users type to filter, arrow keys to select, Enter to execute, Esc to dismiss. This is the discoverability layer that makes every feature accessible to non-developers (security analysts, SREs, etc.) without memorizing vim keys. Power users learn shortcuts naturally because they're displayed next to each command. Also enables actions that don't deserve a dedicated keybinding (export, go-to-line, etc.). We already have `nucleo-matcher` for fuzzy matching, the help overlay rendering pattern, and `AppMode` for modal input. Touches: `app.rs`, `ui.rs`, `main.rs`.
- **Horizontal scroll** — with wrap OFF, long lines truncate silently and there's no way to see the rest. Add `h`/`l` (or Left/Right arrow) horizontal panning with an `h_scroll: usize` offset in `App`. The `Paragraph` widget supports `.scroll((v_offset, h_offset))`. Show a `Col: N` indicator in the status bar when offset > 0. Touches: `app.rs`, `ui.rs`, `main.rs`.


## Medium Priority

- **Stats bar** — `parsed_lines` already has `level` on every entry. Scan once → `HashMap<LogLevel, usize>` counts. Render as a compact colored row above the status bar: `E:42 W:130 I:1204`. Click a count to filter by that level (mouse integration already exists). Touches: `app.rs`, `ui.rs`.
- **Contextual lines** — show N lines before/after each filter match (like `grep -C`). `filtered_indices` is already a `Vec<usize>` — expand each index into a range, merge overlapping ranges, render separators (`---`) between groups. Toggle with a key or `--context N` flag. Touches: `filter.rs`, `app.rs`, `ui.rs`.
- **Regex filter mode** — currently filter is substring-only (with fuzzy fallback). Allow toggling to regex mode, e.g. pressing `Ctrl-R` while in filter mode switches the `/` prompt to `r/`. Use the `regex` crate already in deps. Show `[regex]` in status bar when active. Touches: `filter.rs`, `app.rs`, `ui.rs`, `main.rs`.
- **Bookmarks** — mark lines with `m`, jump between bookmarks with `'` (next) and `"` (prev). Store `BTreeSet<usize>` in `App`. Render a marker glyph (e.g. `>`) in the gutter for bookmarked lines. Touches: `app.rs`, `main.rs`, `ui.rs`.

## Lower Priority

- **Field columns** — structured table view for JSON/logfmt logs (timestamp | level | message in aligned columns). Toggle with `c`. Requires measuring column widths and a different rendering path (ratatui `Table` widget). Especially useful for structured logs where the one-line view is cluttered.
- **Export filtered view** — write the current filtered/highlighted output to a file. `--output` flag or `s` to save interactively. Write raw lines (for piping) or ANSI-colored output (for sharing). Touches: `main.rs`, `app.rs`.
- **Tail multiple files** — `lumolog file1.log file2.log` merges lines, color-coded by source. Useful for microservices debugging. Needs a source multiplexer and per-file color assignment.
- **Config file** — `~/.config/lumolog/config.toml` for custom colors, default keybindings, default wrap on/off, default context lines, etc. Use the `dirs` crate for XDG paths.

## Stretch / Future

- **Time range filtering (Kibana-style)** — visual sparkline density bar at the top of the TUI:
  ```
  ▁▁▂▁▁▁▃▂▁▁▁▁▇█▆▃▁▁▁▂▁▁
  14:00     15:00     16:00     17:00     18:00
  ```
  Spikes show where bursts of log activity happened. Three ways to select a time range:
  - **Keyboard**: press `t` to enter time mode, left/right arrows to move cursor on the sparkline, `[` to mark start, `]` to mark end, Enter to apply, Esc to cancel.
  - **Mouse**: click and drag across the sparkline bar to select a range.
  - **Quick presets**: in time mode, press `1`=last 5m, `2`=last 15m, `3`=last 1h, `4`=last 24h.
  Selected range highlights on the sparkline:
  ```
  ▁▁▂▁▁▁▃▂▁▁▁▁[▇█▆▃]▁▁▁▂▁▁
  14:00     15:00  [16:15—16:45]  17:00     18:00
  ```
  Logs below filter to only the selected time window. Since we already parse timestamps from every format, this works out of the box.
- **Error grouping** — cluster similar error messages (fuzzy dedup). Show a count badge and expand to see individual occurrences. Great for noisy logs with thousands of the same stack trace.
- **Mouse extras** — click line to bookmark, drag to select & copy text region, click stats bar counts to filter by level.
- **Live syntax highlighting in filter bar** — as the user types a filter pattern, colorize the input to show regex groups or highlight the pattern in the visible log lines in real-time (already partially done with search highlighting, but could be smoother).
- **Plugin system** — custom parsers/highlighters loaded from a config directory. Each plugin is a TOML file with regex patterns and color mappings, so users can add support for proprietary log formats without modifying source.
- **AI explain / summarize** — in cursor mode, `a` sends the current log line to a configured LLM with "explain this log entry" and displays the response in a popup overlay. `A` sends all currently filtered lines with "summarize these logs" for a high-level overview. Configure provider/model/API key in `~/.config/lumolog/config.toml`. Needs a background thread for the HTTP call so the UI doesn't freeze.

---

## Known Bugs / Tech Debt

- **Wrap mode scroll math** — `display_line_count()` returns 1 for non-pretty lines even when wrap is on, because we don't know how many visual lines a wrapped line will produce (it depends on terminal width and line content). This means the viewport may show fewer lines than expected or clip at the bottom. Fix: calculate wrapped line count using `unicode-width` and terminal width, or switch to ratatui's built-in scroll offset for wrapped paragraphs.
- **`scroll_to_bottom` fragility** — sets `scroll_offset = len` and relies on a later `clamp_scroll` call from `set_viewport_height` during render. Works but is implicit. Consider clamping inline.

---

## Done

- ~~**Level filtering**~~ — toggle visibility by level with `v`/`V` keys
- ~~**Inline pattern highlighting**~~ — IPs, URLs, UUIDs, paths, HTTP methods, key=value, quoted strings, etc.
- ~~**Mouse support**~~ — scroll wheel, click-to-filter on tokens, context menu with actions (filter, AbuseIPDB lookup, open URL in browser)
- ~~**Follow mode**~~ — `--follow`/`-f` flag with pause/resume
- ~~**JSON pretty-print**~~ — toggle with `p` key
- ~~**Stdin support**~~ — pipe input via stdin
- ~~**Substring filter**~~ — `/` to search, case-insensitive matching
- ~~**Line numbers**~~ — displayed in gutter
- ~~**Help overlay**~~ — `?` to toggle keybinding reference
- ~~**Fuzzy search**~~ — subsequence fuzzy matching via `nucleo-matcher`. Type "conref" to match "connection refused". Falls back to fuzzy when exact substring returns zero results. Status bar shows `Filter~:` when fuzzy is active.
- ~~**Pretty-print scrolling fix**~~ — scrolling in JSON pretty-print mode now accounts for multi-line expanded entries.
- ~~**JSON compact view with all fields**~~ — extra JSON fields stored in `ParsedLine` and rendered as dimmed `key=value` pairs after the message.
- ~~**Search highlighting**~~ — filter matches highlighted in-place with `bg(Yellow) fg(Black)`. Case-insensitive, works in both normal and pretty-print modes.
- ~~**Wrap toggle**~~ — `w` key toggles line wrapping via ratatui `Paragraph::wrap`. Title bar shows `[wrap]` indicator.
- ~~**Cursor mode**~~ — `Enter` activates cursor mode with highlighted bar, `j`/`k` moves cursor, viewport scrolls to follow, `Esc` exits.
- ~~**Copy to clipboard**~~ — `y` in cursor mode yanks the raw line text to clipboard via `arboard`. Status bar flashes "YANKED" briefly.
- ~~**Filter by similar lines**~~ — `s` in cursor mode computes a structural template (replacing IPs, numbers, UUIDs, URLs, timestamps, hex, paths with `*`) and filters to all lines with matching structure. `Esc` clears the similar filter.
- ~~**Logfmt parser**~~ — `detect_format` recognizes `key=value` logs (3+ pairs per line), `parse_logfmt_line` extracts structured fields (level, timestamp, message, extras). Renders with same compact view as JSON.
- ~~**Docker JSON log wrapper**~~ — `"log"` added to message key lookup, `"time"` to timestamp keys, `"stream"` suppressed from extras, trailing `\n` stripped, level fallback scans inside `log` text for embedded keywords.
- ~~**Apache/Nginx CLF parser**~~ — `LogFormat::AccessLog` detects Common/Combined Log Format via bracket timestamp pattern. Extracts IP, user, method, path, status, bytes, referer, user-agent. Status maps to level (5xx=Error, 4xx=Warn, rest=Info).
- ~~**klog (Kubernetes) parser**~~ — `LogFormat::Klog` detects `^[IWEF]\d{4}` prefix. Extracts level from single-letter prefix (I/W/E/F), timestamp, PID, source location as structured fields.
- ~~**Log4j/Java parser**~~ — `LogFormat::Log4j` detects `timestamp [thread] LEVEL class - message` pattern. Extracts thread and fully-qualified class into extra fields, cleans message.
- ~~**Python logging parser**~~ — `LogFormat::PythonLog` detects `timestamp,ms - module - LEVEL - message` pattern. Extracts module into extra fields, handles comma decimal timestamps and `WARNING`/`CRITICAL` levels.
