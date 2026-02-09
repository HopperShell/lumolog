<p align="center">
  <img src="assets/logo.svg" alt="lumolog" width="700"/>
</p>

<p align="center">
  A terminal-based log analysis tool that surfaces what matters in your logs.
</p>

---

Lumolog is a TUI log viewer built for security and operations teams. Point it at any log file and it automatically detects the format, parses structured fields, colorizes by severity, and gives you interactive tools to search, filter, and investigate — right from your terminal.

- **Auto-detection** of JSON, syslog, and plain text log formats
- **Log level filtering** to cut through the noise and focus on what matters
- **Interactive search** with clickable tokens — click an IP, filter by it instantly
- **JSON pretty-printing** for deeply nested log entries
- **Memory-mapped I/O** for fast handling of large log files
- **Follow mode** for tailing live logs
- **Vim-style navigation** for keyboard-driven workflows

> **Note:** Lumolog is under active development. Features and interfaces may change.
