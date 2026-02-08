use ratatui::Frame;
use ratatui::layout::{Layout, Constraint};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let [main_area, status_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
    ]).areas(area);

    let content_height = main_area.height.saturating_sub(2) as usize;
    app.set_viewport_height(content_height);

    let visible: Vec<Line> = app
        .visible_lines()
        .iter()
        .map(|line| Line::raw(line.as_str()))
        .collect();

    let log_view = Paragraph::new(visible)
        .block(Block::default().borders(Borders::ALL).title("lumolog"));

    frame.render_widget(log_view, main_area);

    let total = app.lines().len();
    let offset = app.scroll_offset();
    let pct = if total == 0 {
        100
    } else {
        ((offset + content_height).min(total) * 100) / total
    };
    let status_text = format!(
        " Line {}-{} of {} ({}%) | q:quit  j/k:scroll  PgUp/PgDn  g/G:top/bottom",
        offset + 1,
        (offset + content_height).min(total),
        total,
        pct
    );
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Black).bg(Color::White));

    frame.render_widget(status, status_area);
}
