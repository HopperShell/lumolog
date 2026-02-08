use ratatui::Frame;
use ratatui::layout::{Layout, Constraint};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::highlighter::{highlight_line, highlight_line_expanded};
use crate::parser::LogFormat;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let filter_height = if app.is_filter_mode() { 1 } else { 0 };

    let [main_area, filter_area, status_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(filter_height),
        Constraint::Length(1),
    ]).areas(area);

    let content_height = main_area.height.saturating_sub(2) as usize;
    app.set_viewport_height(content_height);

    let all_display_lines: Vec<Line> = if app.is_pretty() {
        app.visible_parsed_lines()
            .iter()
            .flat_map(|parsed| highlight_line_expanded(parsed, true))
            .collect()
    } else {
        app.visible_parsed_lines()
            .iter()
            .map(|parsed| highlight_line(parsed))
            .collect()
    };

    let format_label = match app.format() {
        LogFormat::Json => "JSON",
        LogFormat::Syslog => "Syslog",
        LogFormat::Plain => "Plain",
    };
    let pretty_indicator = if app.is_pretty() { " pretty" } else { "" };
    let log_view = Paragraph::new(all_display_lines)
        .block(Block::default().borders(Borders::ALL).title(format!("lumolog [{}{}]", format_label, pretty_indicator)));

    frame.render_widget(log_view, main_area);

    // Render filter bar if in filter mode
    if app.is_filter_mode() {
        let filter_text = format!("/{}", app.filter_pattern());
        let filter_bar = Paragraph::new(filter_text)
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(filter_bar, filter_area);
    }

    // Status bar
    let total = app.total_lines();
    let offset = app.scroll_offset();
    let pct = if total == 0 {
        100
    } else {
        ((offset + content_height).min(total) * 100) / total
    };

    let filter_info = if !app.filter_pattern().is_empty() {
        format!(" | Filter: \"{}\" ({} matches)", app.filter_pattern(), app.total_lines())
    } else {
        String::new()
    };

    let status_text = format!(
        " Line {}-{} of {} ({}%){} | q:quit  j/k:scroll  PgUp/PgDn  g/G:top/bottom  /:filter",
        offset + 1,
        (offset + content_height).min(total),
        total,
        pct,
        filter_info
    );
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Black).bg(Color::White));

    frame.render_widget(status, status_area);
}
