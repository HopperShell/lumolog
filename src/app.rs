use crate::filter::filter_lines;
use crate::highlighter::TokenKind;
use crate::parser::{LogFormat, LogLevel, ParsedLine, detect_format, parse_line};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Filter,
    ContextMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    FilterByValue,
    OpenInBrowser,
    LookupAbuseIPDB,
}

impl MenuAction {
    pub fn label(self) -> &'static str {
        match self {
            MenuAction::FilterByValue => "Filter by this value",
            MenuAction::OpenInBrowser => "Open in browser",
            MenuAction::LookupAbuseIPDB => "Lookup on AbuseIPDB",
        }
    }
}

pub struct ContextMenuState {
    pub token_value: String,
    #[allow(dead_code)]
    pub token_kind: TokenKind,
    pub items: Vec<MenuAction>,
    pub selected: usize,
    pub position: (u16, u16),
}

pub struct App {
    parsed_lines: Vec<ParsedLine>,
    format: LogFormat,
    scroll_offset: usize,
    viewport_height: usize,
    quit: bool,
    mode: AppMode,
    filter_pattern: String,
    filtered_indices: Vec<usize>,
    json_pretty: bool,
    show_help: bool,
    source_name: String,
    follow_mode: bool,
    follow_paused: bool,
    min_level: Option<LogLevel>,
    available_levels: Vec<LogLevel>,
    context_menu: Option<ContextMenuState>,
}

impl App {
    pub fn new(lines: Vec<String>) -> Self {
        let format = detect_format(&lines);
        let parsed_lines: Vec<ParsedLine> =
            lines.iter().map(|line| parse_line(line, format)).collect();
        let filtered_indices = (0..parsed_lines.len()).collect();

        let available_levels: Vec<LogLevel> = {
            let set: BTreeSet<LogLevel> = parsed_lines.iter().filter_map(|l| l.level).collect();
            set.into_iter().collect()
        };

        Self {
            parsed_lines,
            format,
            scroll_offset: 0,
            viewport_height: 24,
            quit: false,
            mode: AppMode::Normal,
            filter_pattern: String::new(),
            filtered_indices,
            json_pretty: false,
            show_help: false,
            source_name: String::from("stdin"),
            follow_mode: false,
            follow_paused: false,
            min_level: None,
            available_levels,
            context_menu: None,
        }
    }

    pub fn total_lines(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn total_lines_unfiltered(&self) -> usize {
        self.parsed_lines.len()
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
        self.clamp_scroll();
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
        self.clamp_scroll();
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_height.saturating_sub(2));
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_height.saturating_sub(2));
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.filtered_indices.len();
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    fn clamp_scroll(&mut self) {
        let max = self
            .filtered_indices
            .len()
            .saturating_sub(self.viewport_height);
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    /// Returns (original_line_number, &ParsedLine) pairs for visible lines
    pub fn visible_parsed_lines_numbered(&self) -> Vec<(usize, &ParsedLine)> {
        let start = self.scroll_offset;
        let end = (start + self.viewport_height).min(self.filtered_indices.len());
        self.filtered_indices[start..end]
            .iter()
            .map(|&i| (i + 1, &self.parsed_lines[i])) // 1-indexed
            .collect()
    }

    pub fn format(&self) -> LogFormat {
        self.format
    }

    // Pretty-print methods

    pub fn toggle_pretty(&mut self) {
        self.json_pretty = !self.json_pretty;
    }

    pub fn is_pretty(&self) -> bool {
        self.json_pretty
    }

    // Help overlay methods

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn show_help(&self) -> bool {
        self.show_help
    }

    // Source name methods

    pub fn set_source_name(&mut self, name: String) {
        self.source_name = name;
    }

    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    // Filter mode methods

    pub fn is_filter_mode(&self) -> bool {
        self.mode == AppMode::Filter
    }

    pub fn filter_pattern(&self) -> &str {
        &self.filter_pattern
    }

    pub fn enter_filter_mode(&mut self) {
        self.mode = AppMode::Filter;
    }

    pub fn exit_filter_mode(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn clear_filter(&mut self) {
        self.filter_pattern.clear();
        self.recompute_filter();
        self.mode = AppMode::Normal;
    }

    pub fn filter_input(&mut self, c: char) {
        self.filter_pattern.push(c);
        self.recompute_filter();
    }

    pub fn filter_backspace(&mut self) {
        self.filter_pattern.pop();
        self.recompute_filter();
    }

    fn recompute_filter(&mut self) {
        self.filtered_indices =
            filter_lines(&self.parsed_lines, &self.filter_pattern, self.min_level);
        self.scroll_offset = 0;
    }

    // Follow mode methods

    /// Returns true if the scroll position is at or past the bottom of the content.
    pub fn is_at_bottom(&self) -> bool {
        let max = self
            .filtered_indices
            .len()
            .saturating_sub(self.viewport_height);
        self.scroll_offset >= max
    }

    /// Parse and append new lines, preserving scroll position.
    /// Auto-scrolls to bottom if the user was already at the bottom.
    pub fn append_lines(&mut self, new_raw: Vec<String>) {
        let was_at_bottom = self.is_at_bottom();

        for line in &new_raw {
            let parsed = parse_line(line, self.format);
            self.parsed_lines.push(parsed);
        }

        // Recompute filtered indices from scratch (filter or level filter may be active)
        self.filtered_indices =
            filter_lines(&self.parsed_lines, &self.filter_pattern, self.min_level);

        if was_at_bottom {
            self.scroll_to_bottom();
        }
    }

    pub fn set_follow_mode(&mut self, enabled: bool) {
        self.follow_mode = enabled;
    }

    pub fn is_follow_mode(&self) -> bool {
        self.follow_mode
    }

    pub fn toggle_follow_pause(&mut self) {
        self.follow_paused = !self.follow_paused;
    }

    pub fn is_follow_paused(&self) -> bool {
        self.follow_paused
    }

    // Level filter methods

    pub fn min_level(&self) -> Option<LogLevel> {
        self.min_level
    }

    /// Raise the minimum level (hide more). Cycles: None → second-lowest → … → highest → None.
    pub fn cycle_level_up(&mut self) {
        if self.available_levels.len() <= 1 {
            return;
        }
        self.min_level = match self.min_level {
            None => {
                // Skip the lowest level (min=lowest ≡ no filter), go to second
                self.available_levels.get(1).copied()
            }
            Some(current) => {
                // Find the next higher level
                match self.available_levels.iter().position(|&l| l == current) {
                    Some(idx) if idx + 1 < self.available_levels.len() => {
                        Some(self.available_levels[idx + 1])
                    }
                    _ => None, // wrap around to show all
                }
            }
        };
        self.recompute_filter();
    }

    /// Lower the minimum level (show more). Cycles: None → highest → … → second-lowest → None.
    pub fn cycle_level_down(&mut self) {
        if self.available_levels.len() <= 1 {
            return;
        }
        self.min_level = match self.min_level {
            None => {
                // Start from the highest level
                self.available_levels.last().copied()
            }
            Some(current) => {
                match self.available_levels.iter().position(|&l| l == current) {
                    Some(idx) if idx > 1 => Some(self.available_levels[idx - 1]),
                    _ => None, // at second-lowest or lowest, wrap to show all
                }
            }
        };
        self.recompute_filter();
    }

    // Context menu methods

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    pub fn context_menu(&self) -> Option<&ContextMenuState> {
        self.context_menu.as_ref()
    }

    pub fn open_context_menu(
        &mut self,
        token_value: String,
        token_kind: TokenKind,
        position: (u16, u16),
    ) {
        let mut items = vec![MenuAction::FilterByValue];
        match token_kind {
            TokenKind::Ip => items.push(MenuAction::LookupAbuseIPDB),
            TokenKind::Url => items.push(MenuAction::OpenInBrowser),
            _ => {}
        }
        self.context_menu = Some(ContextMenuState {
            token_value,
            token_kind,
            items,
            selected: 0,
            position,
        });
        self.mode = AppMode::ContextMenu;
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu = None;
        self.mode = AppMode::Normal;
    }

    pub fn menu_up(&mut self) {
        if let Some(ref mut menu) = self.context_menu {
            menu.selected = menu.selected.saturating_sub(1);
        }
    }

    pub fn menu_down(&mut self) {
        if let Some(ref mut menu) = self.context_menu {
            if menu.selected + 1 < menu.items.len() {
                menu.selected += 1;
            }
        }
    }

    pub fn execute_menu_action(&mut self) -> Option<(MenuAction, String)> {
        let menu = self.context_menu.take()?;
        self.mode = AppMode::Normal;
        let action = menu.items[menu.selected];
        Some((action, menu.token_value))
    }

    pub fn set_filter(&mut self, pattern: String) {
        self.filter_pattern = pattern;
        self.recompute_filter();
    }
}
