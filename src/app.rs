pub struct App {
    lines: Vec<String>,
    scroll_offset: usize,
    viewport_height: usize,
    quit: bool,
}

impl App {
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            lines,
            scroll_offset: 0,
            viewport_height: 24,
            quit: false,
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
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
        if self.lines.len() > self.viewport_height {
            self.scroll_offset = self.lines.len() - self.viewport_height;
        }
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    fn clamp_scroll(&mut self) {
        let max = self.lines.len().saturating_sub(self.viewport_height);
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    pub fn visible_lines(&self) -> &[String] {
        let start = self.scroll_offset;
        let end = (start + self.viewport_height).min(self.lines.len());
        &self.lines[start..end]
    }
}
