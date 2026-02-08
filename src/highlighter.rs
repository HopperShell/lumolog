use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::parser::{LogFormat, LogLevel, ParsedLine};

pub fn highlight_line(parsed: &ParsedLine) -> Line<'_> {
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

fn highlight_plain_line(parsed: &ParsedLine) -> Line<'_> {
    let style = level_style(parsed.level);

    if let Some(ref ts) = parsed.timestamp {
        if let Some(pos) = parsed.raw.find(ts.as_str()) {
            let ts_end = pos + ts.len();
            let (ts_part, rest) = parsed.raw.split_at(ts_end);
            return Line::from(vec![
                Span::styled(ts_part.to_string(), timestamp_style()),
                Span::styled(rest.to_string(), style),
            ]);
        }
    }
    Line::from(Span::styled(parsed.raw.clone(), style))
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

    spans.push(Span::styled(parsed.message.clone(), style));

    Line::from(spans)
}

fn highlight_syslog_line(parsed: &ParsedLine) -> Line<'_> {
    let style = level_style(parsed.level);

    if let Some(ref ts) = parsed.timestamp {
        if let Some(pos) = parsed.raw.find(ts.as_str()) {
            let ts_end = pos + ts.len();
            let (ts_part, rest) = parsed.raw.split_at(ts_end);
            return Line::from(vec![
                Span::styled(ts_part.to_string(), timestamp_style()),
                Span::styled(rest.to_string(), style),
            ]);
        }
    }
    Line::from(Span::styled(parsed.raw.clone(), style))
}
