use crate::filter::filter_lines;
use crate::parser::{detect_format, parse_line, LogFormat, ParsedLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Filter,
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
}

impl App {
    pub fn new(lines: Vec<String>) -> Self {
        let format = detect_format(&lines);
        let parsed_lines: Vec<ParsedLine> = lines
            .iter()
            .map(|line| parse_line(line, format))
            .collect();
        let filtered_indices = (0..parsed_lines.len()).collect();
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
        if self.filtered_indices.len() > self.viewport_height {
            self.scroll_offset = self.filtered_indices.len() - self.viewport_height;
        }
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    fn clamp_scroll(&mut self) {
        let max = self.filtered_indices.len().saturating_sub(self.viewport_height);
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
        self.filtered_indices = filter_lines(&self.parsed_lines, &self.filter_pattern);
        self.scroll_offset = 0;
    }
}
