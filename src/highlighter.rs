use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use regex::Regex;
use std::sync::LazyLock;

use crate::parser::{LogFormat, LogLevel, ParsedLine};

// ---------------------------------------------------------------------------
// Inline pattern regexes (ordered by match priority)
// ---------------------------------------------------------------------------

// 1. URLs
static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https?://[^\s,\]>)"']+"#).unwrap());

// 2. UUIDs
static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b").unwrap()
});

// 3. IPv6 (common representations: full, compressed, loopback, link-local)
static IPV6_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}\b|(?i)\b(?:[0-9a-f]{1,4}:){1,7}:|(?i)\b(?:[0-9a-f]{1,4}:){1,6}:[0-9a-f]{1,4}\b|(?i)::(?:[0-9a-f]{1,4}:){0,5}[0-9a-f]{1,4}\b|\b::1\b|\b::\b").unwrap()
});

// 4. IPv4 (with octet capture groups for validation)
static IPV4_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})(?::\d{1,5})?\b").unwrap()
});

// 5. Pointer/memory addresses (0x hex)
static POINTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b0x[0-9a-fA-F]{4,16}\b").unwrap());

// 6. Unix file paths (at least 2 segments)
static PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\./|~/|/)[\w.\-]+(?:/[\w.\-]+)+").unwrap());

// 7. Unix processes: name[pid]
static UNIX_PROCESS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[a-zA-Z][\w.\-]*\[\d+\]").unwrap());

// 8. HTTP methods
static HTTP_METHOD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)\b").unwrap());

// 9. Key=value pairs
static KEY_VALUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|\s)(\w[\w.]*)=").unwrap());

// 10. Quoted strings
static QUOTED_STR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""[^"]{1,200}""#).unwrap());

// 11. Keywords (boolean/null constants)
static KEYWORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:true|false|null|nil|none|undefined|NaN)\b").unwrap());

// 12. Version numbers (dotted: 2.4.1, 10.15.7 — exactly 3 segments)
static VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bv?\d+\.\d+\.\d+\b").unwrap());

// 13. Numbers (2+ digits, or decimal, or with unit suffix — skip tiny standalone digits)
static NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\d+(?:\.\d+)?(?:ns|µs|us|ms|s|m|h|d|KB|MB|GB|TB|B)\b|\d+(?:\.\d+)?%|\d{2,}(?:\.\d+)?\b|\d+\.\d+\b)")
        .unwrap()
});

// 14. Protocol versions (HTTP/1.1, HTTP/2 — prevent number highlighting)
static PROTOCOL_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"HTTP/\d+(?:\.\d+)?").unwrap());

// 15. Inline dates (ISO-style dates not already captured as leading timestamps)
static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}:\d{2}(?:[.,]\d+)?(?:Z|[+\-]\d{2}:?\d{2})?)?")
        .unwrap()
});

// 16. Level keywords (for plain format badge extraction)
static HIGHLIGHT_LEVEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(TRACE|DEBUG|INFO|NOTICE|WARN(?:ING)?|ERROR|FATAL|CRITICAL|SEVERE|EMERGENCY|EMERG|ALERT|PANIC)\b").unwrap()
});

// ---------------------------------------------------------------------------
// Pattern styles
// ---------------------------------------------------------------------------

fn url_style() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::UNDERLINED)
}

fn uuid_style() -> Style {
    Style::default().fg(Color::Magenta)
}

fn ip_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

fn pointer_style() -> Style {
    Style::default().fg(Color::Indexed(208))
}

fn path_style() -> Style {
    Style::default().fg(Color::Indexed(108))
}

fn unix_process_style() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

fn http_method_style() -> Style {
    Style::default()
        .fg(Color::Magenta)
        .add_modifier(Modifier::BOLD)
}

fn key_value_key_style() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

fn quoted_str_style() -> Style {
    Style::default().fg(Color::Yellow)
}

fn keyword_style() -> Style {
    Style::default()
        .fg(Color::LightRed)
        .add_modifier(Modifier::ITALIC)
}

fn number_style() -> Style {
    Style::default().fg(Color::Cyan)
}

fn date_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

// ---------------------------------------------------------------------------
// Token types (for click-to-action)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Url,
    Ip,
    Uuid,
    Path,
    HttpMethod,
    Process,
    KeyValue,
    QuotedString,
    Other,
}

// ---------------------------------------------------------------------------
// Inline pattern tokenizer
// ---------------------------------------------------------------------------

pub struct MatchRegion {
    pub start: usize,
    pub end: usize,
    pub style: Style,
    pub kind: TokenKind,
}

fn overlaps(existing: &[MatchRegion], new: &MatchRegion) -> bool {
    existing
        .iter()
        .any(|r| r.start < new.end && new.start < r.end)
}

/// Validate that an IPv4 regex match has octets in 0-255 range.
fn is_valid_ipv4(caps: &regex::Captures) -> bool {
    (1..=4).all(|i| {
        caps.get(i)
            .and_then(|m| m.as_str().parse::<u16>().ok())
            .is_some_and(|v| v <= 255)
    })
}

/// Helper: collect simple regex matches into regions.
fn collect_matches(
    regex: &Regex,
    text: &str,
    style: Style,
    kind: TokenKind,
    regions: &mut Vec<MatchRegion>,
) {
    for m in regex.find_iter(text) {
        let region = MatchRegion {
            start: m.start(),
            end: m.end(),
            style,
            kind,
        };
        if !overlaps(regions, &region) {
            regions.push(region);
        }
    }
}

/// Collect all recognized pattern regions from `text`.
fn collect_all_regions(text: &str) -> Vec<MatchRegion> {
    let mut regions: Vec<MatchRegion> = Vec::new();

    // Priority order: more specific / structurally significant patterns first.
    // Higher-priority matches claim regions; lower-priority ones skip overlaps.

    // 1. URLs (contain paths, IPs, etc.)
    collect_matches(&URL_RE, text, url_style(), TokenKind::Url, &mut regions);

    // 2. UUIDs
    collect_matches(&UUID_RE, text, uuid_style(), TokenKind::Uuid, &mut regions);

    // 3. IPv6 addresses
    collect_matches(&IPV6_RE, text, ip_style(), TokenKind::Ip, &mut regions);

    // 4. IPv4 addresses (validate octets)
    for caps in IPV4_RE.captures_iter(text) {
        if let Some(m) = caps.get(0) {
            if is_valid_ipv4(&caps) {
                let region = MatchRegion {
                    start: m.start(),
                    end: m.end(),
                    style: ip_style(),
                    kind: TokenKind::Ip,
                };
                if !overlaps(&regions, &region) {
                    regions.push(region);
                }
            }
        }
    }

    // 5. Pointer addresses (0x...)
    collect_matches(
        &POINTER_RE,
        text,
        pointer_style(),
        TokenKind::Other,
        &mut regions,
    );

    // 6. Unix file paths
    collect_matches(&PATH_RE, text, path_style(), TokenKind::Path, &mut regions);

    // 7. Unix processes (sshd[1234])
    collect_matches(
        &UNIX_PROCESS_RE,
        text,
        unix_process_style(),
        TokenKind::Process,
        &mut regions,
    );

    // 8. HTTP methods
    collect_matches(
        &HTTP_METHOD_RE,
        text,
        http_method_style(),
        TokenKind::HttpMethod,
        &mut regions,
    );

    // 9. Key=value pairs (highlight key and '=' only)
    for caps in KEY_VALUE_RE.captures_iter(text) {
        if let Some(key) = caps.get(1) {
            let region = MatchRegion {
                start: key.start(),
                end: key.end() + 1, // +1 for '='
                style: key_value_key_style(),
                kind: TokenKind::KeyValue,
            };
            if !overlaps(&regions, &region) {
                regions.push(region);
            }
        }
    }

    // 10. Quoted strings
    collect_matches(
        &QUOTED_STR_RE,
        text,
        quoted_str_style(),
        TokenKind::QuotedString,
        &mut regions,
    );

    // 11. Keywords (true, false, null, nil, none, undefined, NaN)
    collect_matches(
        &KEYWORD_RE,
        text,
        keyword_style(),
        TokenKind::Other,
        &mut regions,
    );

    // 12. Version numbers (2.4.1 etc. — higher priority than plain numbers)
    collect_matches(
        &VERSION_RE,
        text,
        number_style(),
        TokenKind::Other,
        &mut regions,
    );

    // 13. Protocol versions (HTTP/1.1 — claim region to prevent number highlighting)
    collect_matches(
        &PROTOCOL_VERSION_RE,
        text,
        Style::default(),
        TokenKind::Other,
        &mut regions,
    );

    // 14. Inline dates
    collect_matches(&DATE_RE, text, date_style(), TokenKind::Other, &mut regions);

    // 15. Numbers (lowest priority — avoids coloring parts of IPs, UUIDs, etc.)
    collect_matches(
        &NUMBER_RE,
        text,
        number_style(),
        TokenKind::Other,
        &mut regions,
    );

    // Sort by start position
    regions.sort_by_key(|r| r.start);
    regions
}

/// Split `text` into styled spans, highlighting recognized patterns inline.
/// Unrecognized portions receive `base_style`.
fn tokenize_with_patterns(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let regions = collect_all_regions(text);

    if regions.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }

    let mut spans = Vec::new();
    let mut pos = 0;

    for region in &regions {
        if region.start > pos {
            spans.push(Span::styled(
                text[pos..region.start].to_string(),
                base_style,
            ));
        }
        spans.push(Span::styled(
            text[region.start..region.end].to_string(),
            region.style,
        ));
        pos = region.end;
    }

    if pos < text.len() {
        spans.push(Span::styled(text[pos..].to_string(), base_style));
    }

    spans
}

/// Returns spans with token metadata for click-to-action support.
/// Each entry is (Span, Option<TokenKind>, raw_text).
/// Non-token text has `None` for the kind.
pub fn tokenize_with_metadata(
    text: &str,
    base_style: Style,
) -> Vec<(Span<'static>, Option<TokenKind>, String)> {
    let regions = collect_all_regions(text);

    if regions.is_empty() {
        return vec![(
            Span::styled(text.to_string(), base_style),
            None,
            text.to_string(),
        )];
    }

    let mut result = Vec::new();
    let mut pos = 0;

    for region in &regions {
        if region.start > pos {
            let raw = text[pos..region.start].to_string();
            result.push((Span::styled(raw.clone(), base_style), None, raw));
        }
        let raw = text[region.start..region.end].to_string();
        result.push((
            Span::styled(raw.clone(), region.style),
            Some(region.kind),
            raw,
        ));
        pos = region.end;
    }

    if pos < text.len() {
        let raw = text[pos..].to_string();
        result.push((Span::styled(raw.clone(), base_style), None, raw));
    }

    result
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn highlight_line(parsed: &ParsedLine) -> Line<'_> {
    match parsed.format {
        LogFormat::Json => highlight_json_line(parsed),
        LogFormat::Syslog => highlight_syslog_line(parsed),
        LogFormat::Logfmt
        | LogFormat::Klog
        | LogFormat::Log4j
        | LogFormat::PythonLog
        | LogFormat::AccessLog => highlight_json_line(parsed), // structured formats reuse compact view
        LogFormat::Plain => highlight_plain_line(parsed),
    }
}

fn level_style(level: Option<LogLevel>) -> Style {
    match level {
        Some(LogLevel::Fatal) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        Some(LogLevel::Error) => Style::default().fg(Color::Red),
        Some(LogLevel::Warn) => Style::default().fg(Color::Yellow),
        Some(LogLevel::Info) => Style::default().fg(Color::Green),
        Some(LogLevel::Debug) => Style::default().fg(Color::Indexed(249)), // light gray
        Some(LogLevel::Trace) => Style::default().fg(Color::Indexed(243)), // medium gray
        None => Style::default(),
    }
}

fn timestamp_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn highlight_plain_line(parsed: &ParsedLine) -> Line<'_> {
    let style = level_style(parsed.level);

    if let Some(ref ts) = parsed.timestamp {
        if let Some(pos) = parsed.raw.find(ts.as_str()) {
            let ts_end = pos + ts.len();
            let (ts_part, rest) = parsed.raw.split_at(ts_end);
            let mut spans = vec![Span::styled(ts_part.to_string(), timestamp_style())];

            // Extract level keyword as a bold badge if present
            if parsed.level.is_some() {
                if let Some(level_match) = HIGHLIGHT_LEVEL_RE.find(rest) {
                    let before_level = &rest[..level_match.start()];
                    let level_text = level_match.as_str();
                    let after_level = &rest[level_match.end()..];

                    if !before_level.is_empty() {
                        spans.push(Span::styled(before_level.to_string(), style));
                    }
                    spans.push(Span::styled(
                        level_text.to_string(),
                        style.add_modifier(Modifier::BOLD),
                    ));
                    spans.extend(tokenize_with_patterns(after_level, style));
                    return Line::from(spans);
                }
            }

            spans.extend(tokenize_with_patterns(rest, style));
            return Line::from(spans);
        }
    }
    Line::from(tokenize_with_patterns(&parsed.raw, style))
}

fn highlight_json_line(parsed: &ParsedLine) -> Line<'_> {
    let style = level_style(parsed.level);

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

    spans.push(Span::styled(
        format!("[{}] ", level_str),
        style.add_modifier(Modifier::BOLD),
    ));

    if let Some(ref ts) = parsed.timestamp {
        spans.push(Span::styled(format!("{} ", ts), timestamp_style()));
    }

    spans.extend(tokenize_with_patterns(&parsed.message, style));

    if !parsed.extra_fields.is_empty() {
        let extras: String = parsed
            .extra_fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(" ");
        let dim_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);
        spans.push(Span::styled("  ", dim_style));
        spans.extend(tokenize_with_patterns(&extras, dim_style));
    }

    Line::from(spans)
}

fn highlight_syslog_line(parsed: &ParsedLine) -> Line<'_> {
    let style = level_style(parsed.level);

    if let Some(ref ts) = parsed.timestamp {
        if let Some(pos) = parsed.raw.find(ts.as_str()) {
            let ts_end = pos + ts.len();
            let (ts_part, rest) = parsed.raw.split_at(ts_end);
            let mut spans = vec![Span::styled(ts_part.to_string(), timestamp_style())];
            spans.extend(tokenize_with_patterns(rest, style));
            return Line::from(spans);
        }
    }
    Line::from(tokenize_with_patterns(&parsed.raw, style))
}

/// Overlay search-match highlighting onto an already-styled Line.
/// Finds all case-insensitive occurrences of `pattern` in the concatenated
/// span text, splits spans at match boundaries, and applies bg(Yellow)/fg(Black).
pub fn apply_search_highlight(line: Line<'_>, pattern: &str) -> Line<'static> {
    if pattern.is_empty() {
        return Line::from(
            line.spans
                .iter()
                .map(|s| Span::styled(s.content.to_string(), s.style))
                .collect::<Vec<_>>(),
        );
    }

    let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    let text_lower = full_text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    // Guard: if to_lowercase() changes byte length, skip highlighting
    if text_lower.len() != full_text.len() {
        return Line::from(
            line.spans
                .iter()
                .map(|s| Span::styled(s.content.to_string(), s.style))
                .collect::<Vec<_>>(),
        );
    }

    // Find all match byte ranges
    let mut matches: Vec<(usize, usize)> = Vec::new();
    let mut search_start = 0;
    while let Some(pos) = text_lower[search_start..].find(&pattern_lower) {
        let abs_start = search_start + pos;
        let abs_end = abs_start + pattern_lower.len();
        matches.push((abs_start, abs_end));
        search_start = abs_end;
    }

    if matches.is_empty() {
        return Line::from(
            line.spans
                .iter()
                .map(|s| Span::styled(s.content.to_string(), s.style))
                .collect::<Vec<_>>(),
        );
    }

    let highlight = Style::default().bg(Color::Yellow).fg(Color::Black);
    let mut new_spans: Vec<Span<'static>> = Vec::new();
    let mut text_pos: usize = 0;
    let mut match_idx: usize = 0;

    for span in line.spans.iter() {
        let span_text = span.content.as_ref();
        let span_start = text_pos;
        let span_end = text_pos + span_text.len();
        let mut local_pos = span_start;

        while match_idx < matches.len() && matches[match_idx].0 < span_end {
            let (m_start, m_end) = matches[match_idx];

            let effective_start = m_start.max(span_start);
            if effective_start > local_pos {
                new_spans.push(Span::styled(
                    full_text[local_pos..effective_start].to_string(),
                    span.style,
                ));
            }

            let effective_end = m_end.min(span_end);
            if effective_start < effective_end {
                new_spans.push(Span::styled(
                    full_text[effective_start..effective_end].to_string(),
                    highlight,
                ));
            }

            local_pos = effective_end;

            if m_end <= span_end {
                match_idx += 1;
            } else {
                break;
            }
        }

        if local_pos < span_end {
            new_spans.push(Span::styled(
                full_text[local_pos..span_end].to_string(),
                span.style,
            ));
        }

        text_pos = span_end;
    }

    Line::from(new_spans)
}

/// Returns one or more Lines for a parsed line.
/// In pretty mode for JSON, returns the expanded multi-line JSON.
/// For everything else (or when pretty=false), returns a single line.
pub fn highlight_line_expanded(parsed: &ParsedLine, pretty: bool) -> Vec<Line<'_>> {
    if pretty && parsed.format == LogFormat::Json {
        if let Some(ref pretty_json) = parsed.pretty_json {
            let style = level_style(parsed.level);
            let level_str = match parsed.level {
                Some(LogLevel::Fatal) => "FTL",
                Some(LogLevel::Error) => "ERR",
                Some(LogLevel::Warn) => "WRN",
                Some(LogLevel::Info) => "INF",
                Some(LogLevel::Debug) => "DBG",
                Some(LogLevel::Trace) => "TRC",
                None => "???",
            };

            let mut lines = Vec::new();
            // First line: level badge + separator
            lines.push(Line::from(Span::styled(
                format!("--- [{}] ", level_str),
                style.add_modifier(Modifier::BOLD),
            )));
            // Pretty-printed JSON lines
            for json_line in pretty_json.lines() {
                lines.push(Line::from(Span::styled(format!("  {}", json_line), style)));
            }
            return lines;
        }
    }
    vec![highlight_line(parsed)]
}
