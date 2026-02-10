use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, AppMode};
use crate::highlighter::{
    TokenKind, apply_search_highlight, highlight_line, highlight_line_expanded,
    tokenize_with_metadata,
};
use crate::parser::LogFormat;
use crate::parser::LogLevel;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.tick_yank_flash();

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

    let search_pattern: Option<&str> = if !app.filter_pattern().is_empty() && !app.is_fuzzy() {
        Some(app.filter_pattern())
    } else {
        None
    };

    let cursor_entry_index: Option<usize> = if app.is_cursor_mode() {
        Some(app.cursor_position().saturating_sub(app.scroll_offset()))
    } else {
        None
    };
    let cursor_bg = Color::DarkGray;

    let all_display_lines: Vec<Line> = if app.is_pretty() {
        app.visible_parsed_lines_numbered()
            .iter()
            .enumerate()
            .flat_map(|(entry_idx, (line_num, parsed))| {
                let is_cursor = cursor_entry_index == Some(entry_idx);
                let mut expanded = highlight_line_expanded(parsed, true);
                if let Some(pattern) = search_pattern {
                    expanded = expanded
                        .into_iter()
                        .map(|l| apply_search_highlight(l, pattern))
                        .collect();
                }
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
                if is_cursor {
                    expanded = expanded
                        .into_iter()
                        .map(|l| apply_bg_to_line(l, cursor_bg))
                        .collect();
                }
                expanded
            })
            .collect()
    } else {
        app.visible_parsed_lines_numbered()
            .iter()
            .enumerate()
            .map(|(entry_idx, (line_num, parsed))| {
                let is_cursor = cursor_entry_index == Some(entry_idx);
                let prefix = Span::styled(
                    format!("{:>width$} ", line_num, width = line_num_width),
                    Style::default().fg(Color::DarkGray),
                );
                let mut highlighted = highlight_line(parsed);
                if let Some(pattern) = search_pattern {
                    highlighted = apply_search_highlight(highlighted, pattern);
                }
                highlighted.spans.insert(0, prefix);
                if is_cursor {
                    highlighted = apply_bg_to_line(highlighted, cursor_bg);
                }
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
    let wrap_indicator = if app.is_wrap() { " wrap" } else { "" };
    let mut log_view = Paragraph::new(all_display_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("lumolog [{}{}{}]", format_label, pretty_indicator, wrap_indicator)),
    );
    if app.is_wrap() {
        log_view = log_view.wrap(Wrap { trim: false });
    }

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
        ((offset + app.visible_entry_count()).min(total) * 100) / total
    };

    let mut status_parts = vec![
        format!(" {}", app.source_name()),
        format!("{}", format_label),
        format!("{} lines", total),
    ];

    if app.is_cursor_mode() {
        status_parts.push("CURSOR".to_string());
        if app.show_yank_flash() {
            status_parts.push("YANKED".to_string());
        }
    }

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
        let mode = if app.is_fuzzy() { "~" } else { "" };
        status_parts.push(format!(
            "Filter{}: \"{}\" ({} matches)",
            mode,
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

    // Context menu overlay
    if let Some(menu) = app.context_menu() {
        let items: Vec<Line> = menu
            .items
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let style = if i == menu.selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(format!(" {} ", action.label()), style))
            })
            .collect();

        let menu_width = menu
            .items
            .iter()
            .map(|a| a.label().len() as u16 + 2) // +2 for padding
            .max()
            .unwrap_or(20)
            + 2; // +2 for border
        let menu_height = menu.items.len() as u16 + 2; // +2 for border

        // Clamp position to viewport
        let x = menu.position.0.min(area.width.saturating_sub(menu_width));
        let y = menu.position.1.min(area.height.saturating_sub(menu_height));
        let menu_area = Rect::new(x, y, menu_width, menu_height);

        let menu_block = Paragraph::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Actions")
                .style(Style::default().fg(Color::White).bg(Color::DarkGray)),
        );

        frame.render_widget(Clear, menu_area);
        frame.render_widget(menu_block, menu_area);
    }

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
            Line::from("  /            Filter (fuzzy fallback)"),
            Line::from("  v / V        Cycle log level filter"),
            Line::from("  p            Pretty-print JSON"),
            Line::from("  w            Toggle line wrapping"),
            Line::from("  Enter        Cursor mode (j/k move, y yank, Esc exit)"),
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

/// Given a click coordinate, determine which token (if any) was clicked.
/// Returns the `TokenKind` and the raw matched text.
pub fn token_at_position(
    app: &App,
    column: u16,
    row: u16,
    area: Rect,
) -> Option<(TokenKind, String)> {
    // Recompute the main_area layout the same way render() does
    let filter_height = if app.mode() == AppMode::Filter { 1 } else { 0 };
    let [main_area, _, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(filter_height),
        Constraint::Length(1),
    ])
    .areas(area);

    // The main content area has a 1-cell border on all sides
    let content_x = main_area.x + 1;
    let content_y = main_area.y + 1;
    let content_width = main_area.width.saturating_sub(2);
    let content_height = main_area.height.saturating_sub(2);

    // Check if click is within content bounds
    if column < content_x
        || column >= content_x + content_width
        || row < content_y
        || row >= content_y + content_height
    {
        return None;
    }

    let click_row = (row - content_y) as usize;
    let click_col = (column - content_x) as usize;

    let line_num_width = format!("{}", app.total_lines_unfiltered()).len().max(3);
    let prefix_width = line_num_width + 1; // +1 for the trailing space

    // Determine which parsed line corresponds to this row
    let visible = app.visible_parsed_lines_numbered();

    if app.is_pretty() {
        // Pretty mode: all sub-lines have a same-width prefix (blank for continuations)
        if click_col < prefix_width {
            return None;
        }
        let text_col = click_col - prefix_width;

        let mut display_row = 0;
        for (_line_num, parsed) in &visible {
            let expanded = highlight_line_expanded(parsed, true);
            let row_count = expanded.len();
            if click_row < display_row + row_count {
                let base_style = Style::default();
                let tokens = tokenize_with_metadata(&parsed.message, base_style);
                return find_token_at_col(text_col, &tokens);
            }
            display_row += row_count;
        }
    } else if app.is_wrap() {
        // Wrapped non-pretty: lines may span multiple display rows
        let wrap_width = content_width as usize;
        if wrap_width == 0 {
            return None;
        }

        let mut display_row = 0;
        for (_line_num, parsed) in &visible {
            let content_len: usize = highlight_line(parsed)
                .spans
                .iter()
                .map(|s| s.content.len())
                .sum();
            let line_display_len = prefix_width + content_len;
            let rows = line_display_len.div_ceil(wrap_width).max(1);

            if click_row < display_row + rows {
                let sub_row = click_row - display_row;
                // Map click back to character position in the unwrapped line
                let abs_char_pos = sub_row * wrap_width + click_col;
                if abs_char_pos < prefix_width {
                    return None; // Clicked on line number prefix
                }
                let text_col = abs_char_pos - prefix_width;

                let base_style = Style::default();
                let text_to_tokenize = get_tokenizable_text(parsed);
                let ts_prefix_len = get_timestamp_prefix_len(parsed);
                let tokens = tokenize_with_metadata(text_to_tokenize, base_style);

                let extra_prefix = get_highlight_prefix_len(parsed);
                let adjusted_col = if text_col >= extra_prefix + ts_prefix_len {
                    text_col - extra_prefix - ts_prefix_len
                } else {
                    return None;
                };

                return find_token_at_col(adjusted_col, &tokens);
            }
            display_row += rows;
        }
    } else {
        // Non-pretty, non-wrap: 1 row per visible line
        if click_col < prefix_width {
            return None;
        }
        let text_col = click_col - prefix_width;

        if click_row < visible.len() {
            let (_line_num, parsed) = &visible[click_row];
            let base_style = Style::default();
            let text_to_tokenize = get_tokenizable_text(parsed);
            let ts_prefix_len = get_timestamp_prefix_len(parsed);
            let tokens = tokenize_with_metadata(text_to_tokenize, base_style);

            let extra_prefix = get_highlight_prefix_len(parsed);
            let adjusted_col = if text_col >= extra_prefix + ts_prefix_len {
                text_col - extra_prefix - ts_prefix_len
            } else {
                return None;
            };

            return find_token_at_col(adjusted_col, &tokens);
        }
    }

    None
}

/// Find which token in the metadata list covers the given column offset.
fn find_token_at_col(
    col: usize,
    tokens: &[(ratatui::text::Span<'static>, Option<TokenKind>, String)],
) -> Option<(TokenKind, String)> {
    let mut pos = 0;
    for (_span, kind, raw) in tokens {
        let end = pos + raw.len();
        if col >= pos && col < end {
            return kind.map(|k| (k, raw.clone()));
        }
        pos = end;
    }
    None
}

/// Get the text that `tokenize_with_patterns` is called with for a parsed line.
fn get_tokenizable_text(parsed: &crate::parser::ParsedLine) -> &str {
    match parsed.format {
        LogFormat::Json => &parsed.message,
        LogFormat::Plain | LogFormat::Syslog => {
            if let Some(ref ts) = parsed.timestamp {
                if let Some(pos) = parsed.raw.find(ts.as_str()) {
                    let ts_end = pos + ts.len();
                    return &parsed.raw[ts_end..];
                }
            }
            &parsed.raw
        }
    }
}

/// Returns the character length of the timestamp prefix for plain/syslog lines.
fn get_timestamp_prefix_len(parsed: &crate::parser::ParsedLine) -> usize {
    match parsed.format {
        LogFormat::Json => 0, // JSON timestamp is handled in extra prefix
        LogFormat::Plain | LogFormat::Syslog => {
            if let Some(ref ts) = parsed.timestamp {
                if let Some(pos) = parsed.raw.find(ts.as_str()) {
                    return pos + ts.len();
                }
            }
            0
        }
    }
}

/// Returns the character length of extra prefix spans added by highlight_*_line.
/// For JSON: "[LVL] " (6) + timestamp + space if present.
/// For plain/syslog: 0 (timestamp is part of raw text, handled by ts_prefix_len).
fn get_highlight_prefix_len(parsed: &crate::parser::ParsedLine) -> usize {
    match parsed.format {
        LogFormat::Json => {
            let level_len = 6; // "[XXX] "
            let ts_len = parsed
                .timestamp
                .as_ref()
                .map(|ts| ts.len() + 1) // +1 for trailing space
                .unwrap_or(0);
            level_len + ts_len
        }
        LogFormat::Plain | LogFormat::Syslog => 0,
    }
}

/// Overlay a background color on every span in a line, preserving existing fg/modifiers.
fn apply_bg_to_line(line: Line<'_>, bg: Color) -> Line<'static> {
    Line::from(
        line.spans
            .iter()
            .map(|span| Span::styled(span.content.to_string(), span.style.bg(bg)))
            .collect::<Vec<_>>(),
    )
}

/// Check if a click position lands on a context menu item.
/// Returns the 0-based item index if so.
pub fn menu_item_at_position(app: &App, column: u16, row: u16, area: Rect) -> Option<usize> {
    let menu = app.context_menu()?;

    let menu_width = menu
        .items
        .iter()
        .map(|a| a.label().len() as u16 + 2)
        .max()
        .unwrap_or(20)
        + 2;
    let menu_height = menu.items.len() as u16 + 2;

    let x = menu.position.0.min(area.width.saturating_sub(menu_width));
    let y = menu.position.1.min(area.height.saturating_sub(menu_height));

    // Content area is inside the border: (x+1, y+1) to (x+w-2, y+h-2)
    let content_x = x + 1;
    let content_y = y + 1;
    let content_bottom = y + menu_height - 1;

    if column >= content_x
        && column < x + menu_width - 1
        && row >= content_y
        && row < content_bottom
    {
        let item_index = (row - content_y) as usize;
        if item_index < menu.items.len() {
            return Some(item_index);
        }
    }
    None
}
