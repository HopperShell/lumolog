use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;
use crate::highlighter::{highlight_line, highlight_line_expanded};
use crate::parser::LogFormat;
use crate::parser::LogLevel;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let filter_height = if app.is_filter_mode() { 1 } else { 0 };

    let [main_area, filter_area, status_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(filter_height),
        Constraint::Length(1),
    ])
    .areas(area);

    let content_height = main_area.height.saturating_sub(2) as usize;
    app.set_viewport_height(content_height);

    // Compute line number width from total line count
    let line_num_width = format!("{}", app.total_lines_unfiltered()).len().max(3);

    let all_display_lines: Vec<Line> = if app.is_pretty() {
        app.visible_parsed_lines_numbered()
            .iter()
            .flat_map(|(line_num, parsed)| {
                let mut expanded = highlight_line_expanded(parsed, true);
                // Add line number prefix only to the first line of each expanded group
                if let Some(first) = expanded.first_mut() {
                    let prefix = Span::styled(
                        format!("{:>width$} ", line_num, width = line_num_width),
                        Style::default().fg(Color::DarkGray),
                    );
                    first.spans.insert(0, prefix);
                }
                // Add blank prefix to continuation lines for alignment
                for line in expanded.iter_mut().skip(1) {
                    let blank_prefix = Span::styled(
                        format!("{:>width$} ", "", width = line_num_width),
                        Style::default().fg(Color::DarkGray),
                    );
                    line.spans.insert(0, blank_prefix);
                }
                expanded
            })
            .collect()
    } else {
        app.visible_parsed_lines_numbered()
            .iter()
            .map(|(line_num, parsed)| {
                let prefix = Span::styled(
                    format!("{:>width$} ", line_num, width = line_num_width),
                    Style::default().fg(Color::DarkGray),
                );
                let mut highlighted = highlight_line(parsed);
                highlighted.spans.insert(0, prefix);
                highlighted
            })
            .collect()
    };

    let format_label = match app.format() {
        LogFormat::Json => "JSON",
        LogFormat::Syslog => "Syslog",
        LogFormat::Plain => "Plain",
    };
    let pretty_indicator = if app.is_pretty() { " pretty" } else { "" };
    let log_view = Paragraph::new(all_display_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("lumolog [{}{}]", format_label, pretty_indicator)),
    );

    frame.render_widget(log_view, main_area);

    // Render filter bar if in filter mode
    if app.is_filter_mode() {
        let filter_text = format!("/{}", app.filter_pattern());
        let filter_bar = Paragraph::new(filter_text).style(Style::default().fg(Color::Cyan));
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

    let mut status_parts = vec![
        format!(" {}", app.source_name()),
        format!("{}", format_label),
        format!("{} lines", total),
    ];

    if app.is_follow_mode() {
        if app.is_follow_paused() {
            status_parts.push("PAUSED".to_string());
        } else {
            status_parts.push("FOLLOWING".to_string());
        }
    }

    if let Some(min_level) = app.min_level() {
        status_parts.push(format!("Level: {}+", min_level.short_name()));
    }

    if !app.filter_pattern().is_empty() {
        status_parts.push(format!(
            "Filter: \"{}\" ({} matches)",
            app.filter_pattern(),
            total
        ));
    }

    status_parts.push(format!("{}%", pct));

    let status_text = status_parts.join(" | ");

    // Build styled status bar with colored level indicator
    let status = if let Some(min_level) = app.min_level() {
        let level_label = format!("Level: {}+", min_level.short_name());
        let level_color = match min_level {
            LogLevel::Fatal => Color::Red,
            LogLevel::Error => Color::Red,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Info => Color::Green,
            LogLevel::Debug | LogLevel::Trace => Color::DarkGray,
        };
        // Find where the level part is in the status text and colorize just that part
        if let Some(pos) = status_text.find(&level_label) {
            let before = &status_text[..pos];
            let after = &status_text[pos + level_label.len()..];
            Paragraph::new(Line::from(vec![
                Span::styled(before, Style::default().fg(Color::Black).bg(Color::White)),
                Span::styled(
                    level_label,
                    Style::default().fg(level_color).bg(Color::White),
                ),
                Span::styled(after, Style::default().fg(Color::Black).bg(Color::White)),
            ]))
        } else {
            Paragraph::new(status_text).style(Style::default().fg(Color::Black).bg(Color::White))
        }
    } else {
        Paragraph::new(status_text).style(Style::default().fg(Color::Black).bg(Color::White))
    };

    frame.render_widget(status, status_area);

    // Help overlay
    if app.show_help() {
        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Keybindings",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  q / Esc      Quit"),
            Line::from("  j / k        Scroll down/up"),
            Line::from("  PgUp / PgDn  Page up/down"),
            Line::from("  g / G        Top / Bottom"),
            Line::from("  /            Filter"),
            Line::from("  v / V        Cycle log level filter"),
            Line::from("  p            Pretty-print JSON"),
            Line::from("  Space        Pause/resume (-f mode)"),
            Line::from("  ?            Toggle this help"),
            Line::from(""),
        ];

        // Center the overlay
        let help_width = 40u16;
        let help_height = help_text.len() as u16 + 2; // +2 for border
        let x = (area.width.saturating_sub(help_width)) / 2;
        let y = (area.height.saturating_sub(help_height)) / 2;
        let help_area = Rect::new(x, y, help_width, help_height);

        let help_block = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::White).bg(Color::Black));

        frame.render_widget(Clear, help_area);
        frame.render_widget(help_block, help_area);
    }
}
