# Lumolog Feature Ideas

Core differentiator vs tailspin: tailspin makes text pretty. Lumolog *understands* your logs.

## Done

- ~~**Level filtering**~~ - toggle visibility by level with `v`/`V` keys
- ~~**Inline pattern highlighting**~~ - IPs, URLs, UUIDs, paths, HTTP methods, key=value, quoted strings, etc.
- ~~**Mouse support**~~ - scroll wheel, click-to-filter on tokens, context menu with actions (filter, AbuseIPDB lookup, open URL in browser)
- ~~**Follow mode**~~ - `--follow`/`-f` flag with pause/resume
- ~~**JSON pretty-print**~~ - toggle with `p` key
- ~~**Stdin support**~~ - pipe input via stdin
- ~~**Substring filter**~~ - `/` to search, case-insensitive matching
- ~~**Line numbers**~~ - displayed in gutter
- ~~**Help overlay**~~ - `?` to toggle keybinding reference

## High Priority

- **Fuzzy search** - replace exact substring filter with fuzzy matching (e.g. `nucleo` or `fuzzy-matcher` crate). Type "conref" and match "connection refused". Fall back to fuzzy when exact match returns zero results, so exact matches stay fast and predictable.
- **Logfmt parser** - dedicated format detection for `key=value` logs (Prometheus, Grafana, Go ecosystem). Currently detected as Plain.

## Medium Priority

- **JSON compact view with all fields** - current JSON view drops extra fields. Instead, show `[ERR] 2024-01-15 Failed to connect to redis  error="Connection refused" host=localhost:6379` — core fields up front, remaining fields as dimmed key=value pairs trailing the message. Nothing lost.
- **Field columns** - structured view for JSON/logfmt logs (timestamp | level | message in aligned columns).
- **Stats bar** - show counts by level (42 errors, 130 warnings) since we already classify every line.
- **Bookmarks** - mark interesting lines with `m`, jump between them with `'`.
- **Multi-file** - view/merge multiple log files, color-coded by source. Useful for distributed systems.
- **Export** - write filtered/highlighted results to file.

## Stretch / Future

- **Time range filtering (Kibana-style)** - visual sparkline density bar at the top of the TUI, two rows:
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
- **Auto-detect more formats** - Apache access logs, klog (Kubernetes), Rails request logs, Python tracebacks.
- **Error grouping** - cluster similar error messages together.
- **Contextual lines** - show N lines before/after each match (like grep -C).
- **Search highlighting** - highlight all matches of filter term in-line (not just filter to matching lines).
- **Wrap toggle** - long lines currently truncate; toggle to wrap.
- **Mouse extras** - we have scroll, click-to-filter, and context menus. More ideas:
  - **Click line to bookmark** - click the gutter/margin to toggle a bookmark on a line.
  - **Drag to select & copy** - select a range of text across lines, copy to clipboard.
  - **Click stats bar counts** - if stats bar is implemented, clicking a level count (e.g. "42 errors") filters to that level.
- **Config file** - customizable colors, keybindings.
