//! Test helpers for verifying the lumolog pipeline without the TUI.
//!
//! Provides convenience functions to run log data through
//! Source → Parser → Highlighter and assert on the results.

#![allow(dead_code)]

use lumolog::highlighter::highlight_line;
use lumolog::parser::{LogFormat, LogLevel, ParsedLine, detect_format, parse_line};
use lumolog::source::FileSource;
use ratatui::style::{Color, Modifier};
use ratatui::text::{Line, Span};

// ---------------------------------------------------------------------------
// Pipeline: load file → detect format → parse all lines → highlight
// ---------------------------------------------------------------------------

/// Result of running a log file through the full pipeline.
pub struct PipelineResult {
    pub format: LogFormat,
    pub parsed: Vec<ParsedLine>,
    pub highlighted: Vec<Line<'static>>,
}

/// Run a test data file through the full pipeline: load → detect → parse → highlight.
pub fn pipeline(path: &str) -> PipelineResult {
    let source = FileSource::open(path).expect("failed to open test file");
    let raw_lines: Vec<String> = source.lines().to_vec();
    let format = detect_format(&raw_lines);
    let parsed: Vec<ParsedLine> = raw_lines.iter().map(|l| parse_line(l, format)).collect();
    let highlighted: Vec<Line<'static>> = parsed
        .iter()
        .map(|p| {
            let line = highlight_line(p);
            // Convert to owned Line<'static> so it outlives the borrow
            Line::from(
                line.spans
                    .iter()
                    .map(|s| Span::styled(s.content.to_string(), s.style))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    PipelineResult {
        format,
        parsed,
        highlighted,
    }
}

/// Run raw lines (not from a file) through detect → parse → highlight.
pub fn pipeline_from_lines(lines: &[&str]) -> PipelineResult {
    let raw_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    let format = detect_format(&raw_lines);
    let parsed: Vec<ParsedLine> = raw_lines.iter().map(|l| parse_line(l, format)).collect();
    let highlighted: Vec<Line<'static>> = parsed
        .iter()
        .map(|p| {
            let line = highlight_line(p);
            Line::from(
                line.spans
                    .iter()
                    .map(|s| Span::styled(s.content.to_string(), s.style))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    PipelineResult {
        format,
        parsed,
        highlighted,
    }
}

// ---------------------------------------------------------------------------
// Span query helpers
// ---------------------------------------------------------------------------

/// Get all spans from a highlighted line that have a specific foreground color.
pub fn spans_with_color<'a>(line: &'a Line<'_>, color: Color) -> Vec<&'a Span<'a>> {
    line.spans
        .iter()
        .filter(|s| s.style.fg == Some(color))
        .collect()
}

/// Check if a highlighted line contains a span with the given text and foreground color.
pub fn has_span(line: &Line<'_>, text: &str, color: Color) -> bool {
    line.spans
        .iter()
        .any(|s| s.content.contains(text) && s.style.fg == Some(color))
}

/// Check if any span in the line contains the given text (any color).
pub fn has_text(line: &Line<'_>, text: &str) -> bool {
    line.spans.iter().any(|s| s.content.contains(text))
}

/// Get the full text content of a highlighted line (all spans concatenated).
pub fn line_text(line: &Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.to_string()).collect()
}

/// Check if any span in the line has the given foreground color.
pub fn has_color(line: &Line<'_>, color: Color) -> bool {
    line.spans.iter().any(|s| s.style.fg == Some(color))
}

/// Get all unique foreground colors used in a line.
pub fn colors_in_line(line: &Line<'_>) -> Vec<Color> {
    let mut colors: Vec<Color> = line.spans.iter().filter_map(|s| s.style.fg).collect();
    colors.dedup();
    colors
}

/// Check if a highlighted line contains a span with the given text, foreground color, and modifier.
pub fn has_span_with_modifier(
    line: &Line<'_>,
    text: &str,
    color: Color,
    modifier: Modifier,
) -> bool {
    line.spans.iter().any(|s| {
        s.content.contains(text)
            && s.style.fg == Some(color)
            && s.style.add_modifier.contains(modifier)
    })
}

/// Check if a span with the given text exists and does NOT have the given foreground color.
pub fn has_span_without_color(line: &Line<'_>, text: &str, color: Color) -> bool {
    line.spans
        .iter()
        .any(|s| s.content.contains(text) && s.style.fg != Some(color))
}

/// Check if a highlighted line contains a span with the given text and background color.
pub fn has_span_with_bg(line: &Line<'_>, text: &str, bg: Color) -> bool {
    line.spans
        .iter()
        .any(|s| s.content.contains(text) && s.style.bg == Some(bg))
}

/// Dump a highlighted line as a human-readable string showing each span's text and color.
/// Useful for debugging test failures.
pub fn debug_spans(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|s| {
            let color = match s.style.fg {
                Some(c) => format!("{:?}", c),
                None => "default".to_string(),
            };
            format!("[{}: {:?}]", color, s.content.as_ref())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Level assertion helpers
// ---------------------------------------------------------------------------

/// Assert that a parsed line has the expected log level.
pub fn assert_level(parsed: &ParsedLine, expected: Option<LogLevel>, line_num: usize) {
    assert_eq!(
        parsed.level, expected,
        "Line {}: expected level {:?}, got {:?} (raw: {:?})",
        line_num, expected, parsed.level, parsed.raw
    );
}

/// Assert that an error-level line is colored red.
pub fn assert_error_is_red(line: &Line<'_>, line_num: usize) {
    assert!(
        has_color(line, Color::Red),
        "Line {}: error line should have red spans. Spans: {}",
        line_num,
        debug_spans(line)
    );
}

/// Assert that a warn-level line is colored yellow.
pub fn assert_warn_is_yellow(line: &Line<'_>, line_num: usize) {
    assert!(
        has_color(line, Color::Yellow),
        "Line {}: warn line should have yellow spans. Spans: {}",
        line_num,
        debug_spans(line)
    );
}

/// Assert that an info-level line is colored green.
pub fn assert_info_is_green(line: &Line<'_>, line_num: usize) {
    assert!(
        has_color(line, Color::Green),
        "Line {}: info line should have green spans. Spans: {}",
        line_num,
        debug_spans(line)
    );
}

/// Assert that a debug-level line is colored with Indexed(249) (light gray).
pub fn assert_debug_is_light_gray(line: &Line<'_>, line_num: usize) {
    assert!(
        has_color(line, Color::Indexed(249)),
        "Line {}: debug line should have Indexed(249) spans. Spans: {}",
        line_num,
        debug_spans(line)
    );
}

/// Assert that a trace-level line is colored with Indexed(243) (medium gray).
pub fn assert_trace_is_medium_gray(line: &Line<'_>, line_num: usize) {
    assert!(
        has_color(line, Color::Indexed(243)),
        "Line {}: trace line should have Indexed(243) spans. Spans: {}",
        line_num,
        debug_spans(line)
    );
}

/// Assert level color mapping is correct for a given parsed+highlighted line pair.
pub fn assert_level_color(parsed: &ParsedLine, line: &Line<'_>, line_num: usize) {
    match parsed.level {
        Some(LogLevel::Fatal) | Some(LogLevel::Error) => assert_error_is_red(line, line_num),
        Some(LogLevel::Warn) => assert_warn_is_yellow(line, line_num),
        Some(LogLevel::Info) => assert_info_is_green(line, line_num),
        Some(LogLevel::Debug) => assert_debug_is_light_gray(line, line_num),
        Some(LogLevel::Trace) => assert_trace_is_medium_gray(line, line_num),
        None => {} // no level = no specific color requirement
    }
}
