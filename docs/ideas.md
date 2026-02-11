# Lumolog Feature Ideas

Core differentiator vs tailspin: tailspin makes text pretty. Lumolog *understands* your logs.

## High Priority

Polish the core UX before adding big features. These are low-medium effort and make the tool feel complete.

- **Fix Esc behavior** — Esc in normal mode quits the app, which is hostile. Users expect "go back", not "exit". Change: Esc always means "cancel/back" (exit cursor, close palette, clear filter, clear similar). Only `q` quits. Low effort, high trust gain.
- **Go-to-line** — no way to jump to a specific line number. Stack traces, error messages, and cross-referencing all need this. Add a `GoToLine` action in the command palette that opens a small input prompt. Touches: `command.rs`, `app.rs`, `ui.rs`, `main.rs`.
- **Incremental search (n/N jump)** — filter mode hides non-matching lines entirely. Add a search mode where matches are highlighted in-place and `n`/`N` jumps to the next/previous match without hiding anything. Like `/pattern` then `n` in less/vim. Touches: `app.rs`, `main.rs`, `ui.rs`.
- **Stats bar** — `parsed_lines` already has `level` on every entry. Scan once -> `HashMap<LogLevel, usize>` counts. Render as a compact colored row above the status bar: `E:42 W:130 I:1204`. Click a count to filter by that level (mouse integration already exists). Touches: `app.rs`, `ui.rs`.
- **Contextual lines** — show N lines before/after each filter match (like `grep -C`). `filtered_indices` is already a `Vec<usize>` — expand each index into a range, merge overlapping ranges, render separators (`---`) between groups. Toggle with a key or `--context N` flag. Touches: `filter.rs`, `app.rs`, `ui.rs`.
- **Stdin follow mode** — `docker logs -f | lumolog -f` should work but doesn't. `StdinSource::read_all()` blocks until EOF, so lumolog hangs forever when the pipe stays open. Fix: read stdin in a streaming way — show what's available immediately, then poll for new lines in the event loop (same pattern as file follow mode with `FollowableSource`). Needs a non-blocking stdin reader or a background thread that feeds lines into the app. Touches: `source.rs`, `main.rs`.

## Medium Priority

- **Multi-file merged timeline** — `lumolog api.log backend.log db.log` merges lines sorted by timestamp, color-coded by source. Killer feature for microservices debugging — correlate events across services in one view. Needs a source multiplexer and per-file color assignment. Already have timestamp parsing for all 8 formats.
- **Histogram view** — press `i` to toggle a log volume histogram by time. Buckets by minute, colored by error/warn/info. Click a bucket to jump to that time range. (Sparkline density bar already covers most of this — this would be a more detailed, multi-line breakdown.)
- **Regex filter mode** — currently filter is substring-only (with fuzzy fallback). Allow toggling to regex mode, e.g. pressing `Ctrl-R` while in filter mode switches the `/` prompt to `r/`. Use the `regex` crate already in deps. Show `[regex]` in status bar when active. Touches: `filter.rs`, `app.rs`, `ui.rs`, `main.rs`.
- **Bookmarks** — mark lines with `m`, jump between bookmarks with `'` (next) and `"` (prev). Store `BTreeSet<usize>` in `App`. Render a marker glyph (e.g. `>`) in the gutter for bookmarked lines. Touches: `app.rs`, `main.rs`, `ui.rs`.

## Lower Priority

- **Field columns** — structured table view for JSON/logfmt logs (timestamp | level | message in aligned columns). Toggle with `c`. Requires measuring column widths and a different rendering path (ratatui `Table` widget). Especially useful for structured logs where the one-line view is cluttered.
- **Export filtered view** — write the current filtered/highlighted output to a file. `--output` flag or `s` to save interactively. Write raw lines (for piping) or ANSI-colored output (for sharing). Touches: `main.rs`, `app.rs`.
- **Config file** — `~/.config/lumolog/config.toml` for custom colors, default keybindings, default wrap on/off, default context lines, etc. Use the `dirs` crate for XDG paths.

## Stretch / Future

- **LQL (Lumolog Query Language)** — a purpose-built log query language that replaces the SQL idea. Press `;` to open a query bar. Pipe-based, reads left to right, uses log-specific vocabulary instead of SQL keywords. Designed so you never have to google the syntax.

  **Why not SQL:** SQL is powerful but wrong-shaped for log exploration. Nobody remembers `GROUP BY` vs `ORDER BY` vs `HAVING` under pressure. JOINs, subqueries, and date functions are arcane. Splunk, Datadog, and Grafana all built custom query languages for exactly this reason — domain-specific beats general-purpose for domain-specific tasks.

  **Core syntax — pipes and filters:**
  ```
  # Every query is a pipeline. Each stage filters or transforms.
  # Start with everything, narrow down left to right.

  errors                              # text search (same as / filter)
  level error                         # filter by level
  last 1h                             # time range
  service auth                        # match field value
  "connection refused"                 # exact phrase

  # Chain with pipes
  level error | last 1h                          # errors in the last hour
  level error | last 1h | group service          # ...grouped by service
  level error | last 1h | group service | top 5  # ...top 5 services
  ```

  **Filter stages** (narrow down what you see):
  ```
  level error warn           # level filter (multiple = OR)
  last 5m / 15m / 1h / 24h  # relative time presets
  since 14:00                # absolute start time
  between 14:00 16:30        # absolute time range
  field service = "auth"     # match a structured field
  field status >= 500        # numeric field comparison
  similar "connection reset" # template matching (like s in cursor mode)
  regex "timeout|refused"    # regex mode
  not "healthcheck"          # exclude lines matching text
  ```

  **Aggregation stages** (summarize data):
  ```
  count                      # total matching lines
  count by minute            # time-bucketed counts (mini histogram)
  count by level             # count per level
  group service              # group by field, show counts
  top                        # top 10 (default)
  top 5                      # top 5
  top 50 service             # top 50 by field
  avg response_time          # numeric field average
  p99 response_time          # percentile
  ```

  **Display/action stages** (what to do with results):
  ```
  fields timestamp level message   # show only these fields (column view)
  sort timestamp desc              # sort results
  tail                             # last 20 lines (default)
  tail 100                         # last 100 lines
  head                             # first 20 lines (default)
  head 100                         # first 100 lines
  export /tmp/errors.log           # save to file
  yank                             # copy to clipboard
  ```

  **Design principle — every argument has a sensible default:**
  Every number and modifier is optional. Defaults are whatever you'd pick 80% of the time:
  ```
  top         → top 10
  head        → head 20
  tail        → tail 20
  last        → last 15m
  count by    → count by minute
  sort        → sort timestamp desc
  group       → group + count, sorted desc
  ```
  You can always be specific (`top 50`, `last 4h`, `count by hour`), but the lazy version should just work. Typing more only narrows or refines — never required.

  **Real-world examples:**
  ```
  # "What's failing in auth right now?"
  level error | last 15m | service auth

  # "Show me the top errors this hour"
  level error | last 1h | group message | top 10

  # "How many requests per minute were there?"
  count by minute

  # "P99 latency for the API service today"
  service api | last 24h | p99 field duration

  # "Export all timeout errors for the incident report"
  "timeout" | level error | between 14:30 15:45 | export ~/incident.log

  # Just poke around — each stage narrows interactively
  level warn error
  ```

  **Implementation approach:**
  - New module `src/lql.rs` — tokenizer + pipeline executor
  - Each stage maps to an operation on `filtered_indices` or a new aggregation pass
  - Most filter stages already exist internally (`recompute_filter`, `filter_by_time_range`, template matching)
  - Aggregation stages are new but straightforward — iterate `filtered_indices`, bucket/count/sort
  - Results display in a temporary overlay or replace the main view (toggle back with Esc)
  - Autocomplete in query bar: field names from parsed data, stage keywords, time presets
  - History with up/down arrows (persist across sessions in `~/.config/lumolog/history`)
  - Query bar shows live result count as you type each stage, like the filter bar

  **Why this wins:**
  - Zero learning curve — reads like English, no SQL to memorize
  - Maps 1:1 to lumolog's existing pipeline architecture
  - Every keyboard shortcut becomes a named stage (so the query bar and keyboard are two interfaces to the same engine)
  - Autocomplete makes it self-documenting
  - Can add an optional AI layer later: natural language → LQL (much easier than NL → SQL, and the generated query is readable/editable)

- **Session persistence** — save bookmarks, filters, scroll position to `~/.lumolog/sessions/{file_hash}.json`. Auto-restore on reopen. Makes lumolog investigation-grade for multi-hour debugging sessions.
- **Error grouping** — cluster similar error messages (fuzzy dedup). Show a count badge and expand to see individual occurrences. Great for noisy logs with thousands of the same stack trace.
- **Mouse extras** — click line to bookmark, drag to select & copy text region, click stats bar counts to filter by level.
- **Live syntax highlighting in filter bar** — as the user types a filter pattern, colorize the input to show regex groups or highlight the pattern in the visible log lines in real-time (already partially done with search highlighting, but could be smoother).
- **Plugin system** — custom parsers/highlighters loaded from a config directory. Each plugin is a TOML file with regex patterns and color mappings, so users can add support for proprietary log formats without modifying source.
- **AI explain / summarize** — in cursor mode, `a` sends the current log line to a configured LLM with "explain this log entry" and displays the response in a popup overlay. `A` sends all currently filtered lines with "summarize these logs" for a high-level overview. Configure provider/model/API key in `~/.config/lumolog/config.toml`. Needs a background thread for the HTTP call so the UI doesn't freeze.

---

## Known Bugs / Tech Debt

- **Wrap mode scroll math** — `display_line_count()` returns 1 for non-pretty lines even when wrap is on, because we don't know how many visual lines a wrapped line will produce (it depends on terminal width and line content). This means the viewport may show fewer lines than expected or clip at the bottom. Fix: calculate wrapped line count using `unicode-width` and terminal width, or switch to ratatui's built-in scroll offset for wrapped paragraphs.
- **`scroll_to_bottom` fragility** — sets `scroll_offset = len` and relies on a later `clamp_scroll` call from `set_viewport_height` during render. Works but is implicit. Consider clamping inline.
- **`format_json_value` doesn't escape inner quotes** — `parser.rs:222` wraps strings with `format!("\"{}\"", s)` without escaping. JSON fields containing `"` produce broken output. Should use `serde_json::to_string(v)`.
- **Duplicated level-to-short-name mapping** — `highlighter.rs` has hardcoded `"ERR"`/`"FTL"` strings instead of calling `LogLevel::short_name()`. Three places to update if levels change.
- **Filter rescans all lines per keystroke** — `recompute_filter()` does a full linear scan on every character typed. Fine for small files, but 500k+ line files will lag. Consider debouncing or incremental filtering.

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
- ~~**Copy to clipboard**~~ — `y` in cursor mode yanks the raw line text to clipboard via `arboard`. Status bar flashes "YANKED" briefly. `Y` yanks all filtered lines.
- ~~**Filter by similar lines**~~ — `s` in cursor mode computes a structural template (replacing IPs, numbers, UUIDs, URLs, timestamps, hex, paths with `*`) and filters to all lines with matching structure. `Esc` clears the similar filter.
- ~~**Logfmt parser**~~ — `detect_format` recognizes `key=value` logs (3+ pairs per line), `parse_logfmt_line` extracts structured fields (level, timestamp, message, extras). Renders with same compact view as JSON.
- ~~**Docker JSON log wrapper**~~ — `"log"` added to message key lookup, `"time"` to timestamp keys, `"stream"` suppressed from extras, trailing `\n` stripped, level fallback scans inside `log` text for embedded keywords.
- ~~**Apache/Nginx CLF parser**~~ — `LogFormat::AccessLog` detects Common/Combined Log Format via bracket timestamp pattern. Extracts IP, user, method, path, status, bytes, referer, user-agent. Status maps to level (5xx=Error, 4xx=Warn, rest=Info).
- ~~**klog (Kubernetes) parser**~~ — `LogFormat::Klog` detects `^[IWEF]\d{4}` prefix. Extracts level from single-letter prefix (I/W/E/F), timestamp, PID, source location as structured fields.
- ~~**Log4j/Java parser**~~ — `LogFormat::Log4j` detects `timestamp [thread] LEVEL class - message` pattern. Extracts thread and fully-qualified class into extra fields, cleans message.
- ~~**Python logging parser**~~ — `LogFormat::PythonLog` detects `timestamp,ms - module - LEVEL - message` pattern. Extracts module into extra fields, handles comma decimal timestamps and `WARNING`/`CRITICAL` levels.
- ~~**Horizontal scroll**~~ — `h`/`l`/Left/Right arrows pan horizontally when wrap is off. Mouse horizontal scroll on supported terminals (Ghostty, Kitty, WezTerm). Status bar shows `Col: N` indicator. Toggling wrap resets scroll to 0.
- ~~**Command palette**~~ — `?` opens a fuzzy-searchable command palette (replaced the static help overlay). Shows all actions with keybindings, type to filter via `nucleo-matcher`, arrow keys to select, Enter to execute. Single command registry in `command.rs` is the source of truth — adding a command is one entry. Includes `y` yank line and `Y` yank all filtered lines.
- ~~**Time range filtering (Kibana-style)**~~ — sparkline density bar at top shows log volume over time. `t` enters time mode with keyboard cursor (`h`/`l` to move, `[`/`]` to mark range, `1`–`4` for presets). Mouse click-drag on sparkline selects range instantly. `Y` yanks all filtered lines from any mode. Composes with text, level, and template filters. Supports 10+ timestamp formats (RFC3339, syslog, klog, epoch, Apache CLF, etc.).
- ~~**Match count in filter bar**~~ — live match count and fuzzy indicator shown inline in filter bar while typing. `/ error  (142 matches)` or `/ conref  (~38 fuzzy)`.
