use crate::command;
use crate::filter::filter_lines;
use crate::highlighter::TokenKind;
use crate::parser::{LogFormat, LogLevel, ParsedLine, detect_format, parse_line};
use crate::timeindex::{
    SparklineData, TimeIndex, TimeModeState, TimeRange, bucket_range_to_time_range,
    build_time_index, compute_sparkline, filter_by_time_range,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Filter,
    ContextMenu,
    Cursor,
    CommandPalette,
    TimeRange,
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
    is_fuzzy: bool,
    json_pretty: bool,
    source_name: String,
    follow_mode: bool,
    follow_paused: bool,
    min_level: Option<LogLevel>,
    available_levels: Vec<LogLevel>,
    context_menu: Option<ContextMenuState>,
    wrap: bool,
    h_scroll: usize,
    cursor_position: usize,
    yank_flash: u8,
    similar_template: Option<String>,
    palette_input: String,
    palette_selected: usize,
    palette_filtered: Vec<usize>,
    time_index: Option<TimeIndex>,
    sparkline_data: Option<SparklineData>,
    time_range: Option<TimeRange>,
    time_mode: Option<TimeModeState>,
    sparkline_visible: bool,
    sparkline_width: usize,
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

        let time_index = {
            let idx = build_time_index(&parsed_lines);
            if idx.has_timestamps() {
                Some(idx)
            } else {
                None
            }
        };
        let sparkline_visible = time_index.is_some();

        Self {
            parsed_lines,
            format,
            scroll_offset: 0,
            viewport_height: 24,
            quit: false,
            mode: AppMode::Normal,
            filter_pattern: String::new(),
            filtered_indices,
            is_fuzzy: false,
            json_pretty: false,
            source_name: String::from("stdin"),
            follow_mode: false,
            follow_paused: false,
            min_level: None,
            available_levels,
            context_menu: None,
            wrap: false,
            h_scroll: 0,
            cursor_position: 0,
            yank_flash: 0,
            similar_template: None,
            palette_input: String::new(),
            palette_selected: 0,
            palette_filtered: (0..command::commands().len()).collect(),
            time_index,
            sparkline_data: None,
            time_range: None,
            time_mode: None,
            sparkline_visible,
            sparkline_width: 0,
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
        let step = self
            .viewport_entries_from(self.scroll_offset)
            .saturating_sub(1);
        self.scroll_down(step);
    }

    pub fn page_up(&mut self) {
        let step = self
            .viewport_entries_from(self.scroll_offset)
            .saturating_sub(1);
        self.scroll_up(step);
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
        let entries_from_end = self.viewport_entries_from_end();
        let max = self.filtered_indices.len().saturating_sub(entries_from_end);
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    /// How many display lines does the entry at `parsed_lines[idx]` produce?
    fn display_line_count(&self, idx: usize) -> usize {
        if self.json_pretty
            && let Some(ref pj) = self.parsed_lines[idx].pretty_json
        {
            return pj.lines().count() + 1; // header + JSON body lines
        }
        1
    }

    /// Starting at filtered entry `start`, count how many entries fit in the viewport.
    fn viewport_entries_from(&self, start: usize) -> usize {
        let mut display_lines = 0;
        let mut count = 0;
        for &idx in &self.filtered_indices[start..] {
            let lines = self.display_line_count(idx);
            display_lines += lines;
            count += 1;
            if display_lines >= self.viewport_height {
                break;
            }
        }
        count
    }

    /// Walking backward from the end, count how many entries fit in the viewport.
    fn viewport_entries_from_end(&self) -> usize {
        let mut display_lines = 0;
        let mut count = 0;
        for &idx in self.filtered_indices.iter().rev() {
            let lines = self.display_line_count(idx);
            display_lines += lines;
            count += 1;
            if display_lines >= self.viewport_height {
                break;
            }
        }
        count
    }

    /// Returns (original_line_number, &ParsedLine) pairs for visible lines
    pub fn visible_parsed_lines_numbered(&self) -> Vec<(usize, &ParsedLine)> {
        let start = self.scroll_offset;
        let count = self.viewport_entries_from(start);
        let end = (start + count).min(self.filtered_indices.len());
        self.filtered_indices[start..end]
            .iter()
            .map(|&i| (i + 1, &self.parsed_lines[i])) // 1-indexed
            .collect()
    }

    /// How many filtered entries are visible in the current viewport.
    pub fn visible_entry_count(&self) -> usize {
        self.viewport_entries_from(self.scroll_offset)
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

    pub fn toggle_wrap(&mut self) {
        self.wrap = !self.wrap;
        self.h_scroll = 0;
    }

    pub fn is_wrap(&self) -> bool {
        self.wrap
    }

    pub fn scroll_right(&mut self, n: usize) {
        self.h_scroll = self.h_scroll.saturating_add(n);
    }

    pub fn scroll_left(&mut self, n: usize) {
        self.h_scroll = self.h_scroll.saturating_sub(n);
    }

    pub fn h_scroll(&self) -> usize {
        self.h_scroll
    }

    // Cursor mode methods

    pub fn enter_cursor_mode(&mut self) {
        self.cursor_position = self.scroll_offset;
        self.mode = AppMode::Cursor;
    }

    pub fn exit_cursor_mode(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn is_cursor_mode(&self) -> bool {
        self.mode == AppMode::Cursor
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    pub fn cursor_line_raw(&self) -> Option<&str> {
        self.filtered_indices
            .get(self.cursor_position)
            .map(|&idx| self.parsed_lines[idx].raw.as_str())
    }

    pub fn all_filtered_lines_raw(&self) -> String {
        self.filtered_indices
            .iter()
            .map(|&idx| self.parsed_lines[idx].raw.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn set_yank_flash(&mut self) {
        self.yank_flash = 3;
    }

    pub fn tick_yank_flash(&mut self) {
        self.yank_flash = self.yank_flash.saturating_sub(1);
    }

    pub fn show_yank_flash(&self) -> bool {
        self.yank_flash > 0
    }

    pub fn cursor_down(&mut self, n: usize) {
        let max = self.filtered_indices.len().saturating_sub(1);
        self.cursor_position = (self.cursor_position + n).min(max);
        self.scroll_to_cursor();
    }

    pub fn cursor_up(&mut self, n: usize) {
        self.cursor_position = self.cursor_position.saturating_sub(n);
        self.scroll_to_cursor();
    }

    fn scroll_to_cursor(&mut self) {
        // Cursor above viewport → scroll up
        if self.cursor_position < self.scroll_offset {
            self.scroll_offset = self.cursor_position;
            return;
        }
        // Cursor below viewport → scroll down
        let visible_count = self.viewport_entries_from(self.scroll_offset);
        let last_visible = self.scroll_offset + visible_count.saturating_sub(1);
        if self.cursor_position > last_visible {
            // Walk backward from cursor to find scroll_offset that makes cursor the last visible entry
            let mut display_lines = 0;
            let mut new_offset = self.cursor_position;
            for i in (0..=self.cursor_position).rev() {
                let idx = self.filtered_indices[i];
                let lines = self.display_line_count(idx);
                if display_lines + lines > self.viewport_height {
                    break;
                }
                display_lines += lines;
                new_offset = i;
            }
            self.scroll_offset = new_offset;
        }
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

    pub fn is_fuzzy(&self) -> bool {
        self.is_fuzzy
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
        let result = filter_lines(&self.parsed_lines, &self.filter_pattern, self.min_level);
        let mut indices = result.indices;
        // Time range filter
        if let (Some(index), Some(range)) = (&self.time_index, &self.time_range) {
            indices = filter_by_time_range(index, range, &indices);
        }
        if let Some(ref tmpl) = self.similar_template {
            indices.retain(|&i| self.parsed_lines[i].template == *tmpl);
        }
        self.filtered_indices = indices;
        self.is_fuzzy = result.is_fuzzy;
        self.scroll_offset = 0;
        if self.mode == AppMode::Cursor {
            self.cursor_position = 0;
        }
    }

    // Follow mode methods

    /// Returns true if the scroll position is at or past the bottom of the content.
    pub fn is_at_bottom(&self) -> bool {
        let entries_from_end = self.viewport_entries_from_end();
        let max = self.filtered_indices.len().saturating_sub(entries_from_end);
        self.scroll_offset >= max
    }

    /// Parse and append new lines, preserving scroll position.
    /// Auto-scrolls to bottom if the user was already at the bottom.
    pub fn append_lines(&mut self, new_raw: Vec<String>) {
        // Re-detect format if app started with no lines (stdin follow mode)
        if self.parsed_lines.is_empty() && !new_raw.is_empty() {
            self.format = detect_format(&new_raw);
        }

        let was_at_bottom = self.is_at_bottom();

        let new_parsed: Vec<ParsedLine> = new_raw
            .iter()
            .map(|line| parse_line(line, self.format))
            .collect();

        // Update time index incrementally
        if let Some(ref mut idx) = self.time_index {
            idx.append(&new_parsed);
            // Recompute sparkline with current width
            if self.sparkline_width > 0 {
                self.sparkline_data = compute_sparkline(idx, self.sparkline_width);
            }
        }

        self.parsed_lines.extend(new_parsed);

        // Recompute filtered indices from scratch (filter or level filter may be active)
        let result = filter_lines(&self.parsed_lines, &self.filter_pattern, self.min_level);
        let mut indices = result.indices;
        if let (Some(index), Some(range)) = (&self.time_index, &self.time_range) {
            indices = filter_by_time_range(index, range, &indices);
        }
        if let Some(ref tmpl) = self.similar_template {
            indices.retain(|&i| self.parsed_lines[i].template == *tmpl);
        }
        self.filtered_indices = indices;
        self.is_fuzzy = result.is_fuzzy;

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

    /// Returns counts of each log level present in the full (unfiltered) dataset.
    /// Sorted by severity: Trace, Debug, Info, Warn, Error, Fatal. Only non-zero levels included.
    pub fn level_counts(&self) -> Vec<(LogLevel, usize)> {
        use std::collections::BTreeMap;
        let mut counts: BTreeMap<LogLevel, usize> = BTreeMap::new();
        for line in &self.parsed_lines {
            if let Some(level) = line.level {
                *counts.entry(level).or_insert(0) += 1;
            }
        }
        counts.into_iter().collect()
    }

    /// Set min_level to the given level, or clear it if already set to that level.
    pub fn set_min_level(&mut self, level: LogLevel) {
        if self.min_level == Some(level) {
            self.min_level = None;
        } else {
            self.min_level = Some(level);
        }
        self.recompute_filter();
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
        if let Some(ref mut menu) = self.context_menu
            && menu.selected + 1 < menu.items.len()
        {
            menu.selected += 1;
        }
    }

    pub fn execute_menu_action(&mut self) -> Option<(MenuAction, String)> {
        let menu = self.context_menu.take()?;
        self.mode = AppMode::Normal;
        let action = menu.items[menu.selected];
        Some((action, menu.token_value))
    }

    pub fn execute_menu_item(&mut self, index: usize) -> Option<(MenuAction, String)> {
        let menu = self.context_menu.take()?;
        self.mode = AppMode::Normal;
        let action = *menu.items.get(index)?;
        Some((action, menu.token_value))
    }

    pub fn set_filter(&mut self, pattern: String) {
        self.filter_pattern = pattern;
        self.recompute_filter();
    }

    /// Filter to lines structurally similar to the current cursor line.
    pub fn filter_by_similar(&mut self) {
        if let Some(&idx) = self.filtered_indices.get(self.cursor_position) {
            let tmpl = self.parsed_lines[idx].template.clone();
            self.similar_template = Some(tmpl);
            self.mode = AppMode::Normal;
            self.recompute_filter();
        }
    }

    pub fn clear_similar(&mut self) {
        self.similar_template = None;
        self.recompute_filter();
    }

    pub fn is_similar_filter(&self) -> bool {
        self.similar_template.is_some()
    }

    // Command palette methods

    pub fn open_palette(&mut self) {
        self.palette_input.clear();
        self.palette_selected = 0;
        self.palette_filtered = (0..command::commands().len()).collect();
        self.mode = AppMode::CommandPalette;
    }

    pub fn close_palette(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn palette_input(&self) -> &str {
        &self.palette_input
    }

    pub fn palette_filtered(&self) -> &[usize] {
        &self.palette_filtered
    }

    pub fn palette_selected(&self) -> usize {
        self.palette_selected
    }

    pub fn palette_type(&mut self, c: char) {
        self.palette_input.push(c);
        self.recompute_palette();
    }

    pub fn palette_backspace(&mut self) {
        self.palette_input.pop();
        self.recompute_palette();
    }

    pub fn palette_up(&mut self) {
        self.palette_selected = self.palette_selected.saturating_sub(1);
    }

    pub fn palette_down(&mut self) {
        if self.palette_selected + 1 < self.palette_filtered.len() {
            self.palette_selected += 1;
        }
    }

    pub fn palette_execute(&mut self) -> Option<command::Action> {
        let &idx = self.palette_filtered.get(self.palette_selected)?;
        let action = command::commands()[idx].action;
        self.close_palette();
        Some(action)
    }

    fn recompute_palette(&mut self) {
        let cmds = command::commands();
        if self.palette_input.is_empty() {
            self.palette_filtered = (0..cmds.len()).collect();
        } else {
            use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
            use nucleo_matcher::{Config, Matcher, Utf32Str};

            let mut matcher = Matcher::new(Config::DEFAULT);
            let pat = Pattern::new(
                &self.palette_input,
                CaseMatching::Ignore,
                Normalization::Smart,
                AtomKind::Fuzzy,
            );
            let mut buf = Vec::new();

            // Collect (index, score) pairs, then sort by score descending
            let mut scored: Vec<(usize, u32)> = cmds
                .iter()
                .enumerate()
                .filter_map(|(i, cmd)| {
                    buf.clear();
                    let haystack = Utf32Str::new(cmd.name, &mut buf);
                    pat.score(haystack, &mut matcher).map(|s| (i, s))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.palette_filtered = scored.into_iter().map(|(i, _)| i).collect();
        }
        self.palette_selected = 0;
    }

    // Time range / sparkline methods

    pub fn is_sparkline_visible(&self) -> bool {
        self.sparkline_visible && self.time_index.is_some()
    }

    pub fn toggle_sparkline(&mut self) {
        self.sparkline_visible = !self.sparkline_visible;
    }

    pub fn sparkline_data(&self) -> Option<&SparklineData> {
        self.sparkline_data.as_ref()
    }

    pub fn time_range(&self) -> Option<&TimeRange> {
        self.time_range.as_ref()
    }

    pub fn time_mode_state(&self) -> Option<&TimeModeState> {
        self.time_mode.as_ref()
    }

    pub fn time_index(&self) -> Option<&TimeIndex> {
        self.time_index.as_ref()
    }

    pub fn set_sparkline_width(&mut self, width: usize) {
        if width != self.sparkline_width && width > 0 {
            self.sparkline_width = width;
            if let Some(index) = &self.time_index {
                self.sparkline_data = compute_sparkline(index, width);
            }
        }
    }

    pub fn enter_time_mode(&mut self) {
        if self.time_index.is_none() || !self.sparkline_visible {
            return;
        }
        let num_buckets = self.sparkline_data.as_ref().map_or(0, |s| s.num_buckets);
        if num_buckets == 0 {
            return;
        }
        self.time_mode = Some(TimeModeState {
            cursor_bucket: num_buckets / 2,
            range_start: None,
            dragging: false,
            drag_start: None,
        });
        self.mode = AppMode::TimeRange;
    }

    pub fn exit_time_mode(&mut self) {
        self.time_mode = None;
        self.mode = AppMode::Normal;
    }

    pub fn time_cursor_left(&mut self, n: usize) {
        if let Some(state) = &mut self.time_mode {
            state.cursor_bucket = state.cursor_bucket.saturating_sub(n);
        }
    }

    pub fn time_cursor_right(&mut self, n: usize) {
        if let (Some(state), Some(sparkline)) = (&mut self.time_mode, &self.sparkline_data) {
            let max = sparkline.num_buckets.saturating_sub(1);
            state.cursor_bucket = (state.cursor_bucket + n).min(max);
        }
    }

    pub fn time_mark_start(&mut self) {
        if let Some(state) = &mut self.time_mode {
            state.range_start = Some(state.cursor_bucket);
        }
    }

    pub fn time_mark_end_and_apply(&mut self) {
        let (start_bucket, end_bucket) = {
            let state = match &self.time_mode {
                Some(s) => s,
                None => return,
            };
            let start = match state.range_start {
                Some(s) => s,
                None => state.cursor_bucket,
            };
            let end = state.cursor_bucket;
            if start <= end {
                (start, end)
            } else {
                (end, start)
            }
        };

        if let Some(sparkline) = &self.sparkline_data
            && let Some(range) = bucket_range_to_time_range(sparkline, start_bucket, end_bucket)
        {
            self.time_range = Some(range);
            self.recompute_filter();
        }
        self.exit_time_mode();
    }

    pub fn time_preset(&mut self, minutes: i64) {
        if let Some(index) = &self.time_index
            && let Some(max_ts) = index.max_ts
        {
            let start = max_ts - chrono::Duration::minutes(minutes);
            self.time_range = Some(TimeRange { start, end: max_ts });
            self.recompute_filter();
        }
        self.exit_time_mode();
    }

    pub fn clear_time_range(&mut self) {
        self.time_range = None;
        self.recompute_filter();
    }

    pub fn time_mouse_down(&mut self, bucket: usize) {
        if self.time_index.is_none() || !self.sparkline_visible {
            return;
        }
        // Enter time mode if not already in it
        if self.mode != AppMode::TimeRange {
            self.enter_time_mode();
        }
        if let Some(state) = &mut self.time_mode {
            state.cursor_bucket = bucket;
            state.dragging = true;
            state.drag_start = Some(bucket);
            state.range_start = Some(bucket);
        }
    }

    pub fn time_mouse_drag(&mut self, bucket: usize) {
        if let Some(state) = &mut self.time_mode
            && state.dragging
        {
            state.cursor_bucket = bucket;
        }
    }

    pub fn time_mouse_up(&mut self, _bucket: usize) {
        let should_apply = self
            .time_mode
            .as_ref()
            .is_some_and(|s| s.dragging && s.drag_start.is_some());
        if should_apply {
            // Mark end and apply
            self.time_mark_end_and_apply();
        }
    }
}
