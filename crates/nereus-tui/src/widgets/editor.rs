//! Code Submission Editor widget with syntax styled rendering and scroll offset tracking.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, StatefulWidget, Widget};

/// Default initial boilerplate code for the student submission buffer.
pub const DEFAULT_CODE_BOILERPLATE: &str = r#"// Nereus Socratic Challenge Solution
// Implement your solution below. Press [Enter] to submit.

pub fn solve() -> Result<(), Box<dyn std::error::Error>> {
    // Socratic Hint: Ensure type safety and memory ownership
    Ok(())
}"#;

/// Maximum snapshots kept in the undo/redo ring.
pub const MAX_UNDO_HISTORY: usize = 64;

/// Reactive state tracking buffer lines, cursor position, and viewport scroll offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorState {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub horizontal_offset: usize,
    pub is_focused: bool,
    pub is_modified: bool,
    pub undo_stack: Vec<(Vec<String>, usize, usize)>,
    pub redo_stack: Vec<(Vec<String>, usize, usize)>,
    pub is_search_open: bool,
    pub search_query: String,
    pub search_matches: Vec<(usize, usize)>,
    pub current_match_index: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new(DEFAULT_CODE_BOILERPLATE)
    }
}

impl EditorState {
    /// Creates a new editor state initialized with the given text.
    pub fn new(initial_code: &str) -> Self {
        let lines: Vec<String> = initial_code.lines().map(|s| s.to_string()).collect();
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };

        Self {
            lines,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            horizontal_offset: 0,
            is_focused: false,
            is_modified: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            is_search_open: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_match_index: 0,
        }
    }

    /// Saves a snapshot of current buffer content and cursor before mutation.
    pub fn save_snapshot(&mut self) {
        if self.undo_stack.len() >= MAX_UNDO_HISTORY {
            self.undo_stack.remove(0);
        }
        self.undo_stack
            .push((self.lines.clone(), self.cursor_line, self.cursor_col));
        self.redo_stack.clear();
    }

    /// Reverts the buffer to the previous historical snapshot.
    pub fn undo(&mut self) -> bool {
        if let Some((prev_lines, prev_line, prev_col)) = self.undo_stack.pop() {
            if self.redo_stack.len() >= MAX_UNDO_HISTORY {
                self.redo_stack.remove(0);
            }
            self.redo_stack
                .push((self.lines.clone(), self.cursor_line, self.cursor_col));
            self.lines = prev_lines;
            self.cursor_line = prev_line.min(self.lines.len().saturating_sub(1));
            self.cursor_col = prev_col.min(self.lines.get(self.cursor_line).map_or(0, |l| l.len()));
            self.is_modified = true;
            true
        } else {
            false
        }
    }

    /// Re-applies a previously undone mutation snapshot.
    pub fn redo(&mut self) -> bool {
        if let Some((next_lines, next_line, next_col)) = self.redo_stack.pop() {
            if self.undo_stack.len() >= MAX_UNDO_HISTORY {
                self.undo_stack.remove(0);
            }
            self.undo_stack
                .push((self.lines.clone(), self.cursor_line, self.cursor_col));
            self.lines = next_lines;
            self.cursor_line = next_line.min(self.lines.len().saturating_sub(1));
            self.cursor_col = next_col.min(self.lines.get(self.cursor_line).map_or(0, |l| l.len()));
            self.is_modified = true;
            true
        } else {
            false
        }
    }

    /// Opens in-buffer search modal.
    pub fn open_search(&mut self) {
        self.is_search_open = true;
        self.update_search_matches();
    }

    /// Closes in-buffer search modal.
    pub fn close_search(&mut self) {
        self.is_search_open = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match_index = 0;
    }

    /// Re-evaluates all occurrences of `search_query` across all lines.
    pub fn update_search_matches(&mut self) {
        self.search_matches.clear();
        if self.search_query.is_empty() {
            return;
        }

        let q = self.search_query.to_lowercase();
        for (line_idx, line) in self.lines.iter().enumerate() {
            let lower = line.to_lowercase();
            let mut start = 0;
            while let Some(pos) = lower[start..].find(&q) {
                let col = start + pos;
                self.search_matches.push((line_idx, col));
                start = col + q.len().max(1);
            }
        }

        if !self.search_matches.is_empty() && self.current_match_index >= self.search_matches.len()
        {
            self.current_match_index = 0;
        }
    }

    /// Jumps cursor to the next match.
    pub fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.current_match_index = (self.current_match_index + 1) % self.search_matches.len();
        let (l, c) = self.search_matches[self.current_match_index];
        self.cursor_line = l;
        self.cursor_col = c;
    }

    /// Jumps cursor to the previous match.
    pub fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.current_match_index == 0 {
            self.current_match_index = self.search_matches.len() - 1;
        } else {
            self.current_match_index -= 1;
        }
        let (l, c) = self.search_matches[self.current_match_index];
        self.cursor_line = l;
        self.cursor_col = c;
    }

    /// Retrieves the complete content of the editor as a single string.
    pub fn get_code(&self) -> String {
        self.lines.join("\n")
    }

    /// Sets the editor buffer to a new code string, resetting cursor and scroll offsets.
    pub fn set_code(&mut self, code: &str) {
        let lines: Vec<String> = code.lines().map(|s| s.to_string()).collect();
        self.lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.horizontal_offset = 0;
        self.is_modified = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Clears the editor text.
    pub fn clear(&mut self) {
        self.save_snapshot();
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.horizontal_offset = 0;
        self.is_modified = true;
    }

    /// Inserts a character at the current cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.save_snapshot();
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
            self.cursor_line = self.lines.len() - 1;
        }

        let line = &mut self.lines[self.cursor_line];
        if self.cursor_col > line.len() {
            self.cursor_col = line.len();
        }

        line.insert(self.cursor_col, c);
        self.cursor_col += 1;
        self.is_modified = true;
    }

    /// Inserts a newline at the current cursor position with smart auto-indentation.
    pub fn insert_newline(&mut self) {
        self.save_snapshot();
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
            self.cursor_line = self.lines.len() - 1;
        }

        let current_line = &self.lines[self.cursor_line];
        let col = self.cursor_col.min(current_line.len());

        // Extract leading whitespace for indentation preservation
        let leading_spaces: String = current_line
            .chars()
            .take_while(|c| c.is_whitespace() && *c != '\n')
            .collect();

        let trimmed_before_cursor = current_line[..col].trim_end();
        let extra_indent = if trimmed_before_cursor.ends_with('{')
            || trimmed_before_cursor.ends_with('(')
            || trimmed_before_cursor.ends_with('[')
        {
            "    "
        } else {
            ""
        };

        let new_indent = format!("{}{}", leading_spaces, extra_indent);
        let remainder = current_line[col..].to_string();

        self.lines[self.cursor_line].truncate(col);
        let new_line = format!("{}{}", new_indent, remainder);
        self.lines.insert(self.cursor_line + 1, new_line);

        self.cursor_line += 1;
        self.cursor_col = new_indent.len();
        self.is_modified = true;
    }

    /// Deletes the character directly preceding the cursor (Backspace).
    pub fn delete_backspace(&mut self) {
        self.save_snapshot();
        if self.cursor_col > 0 {
            if let Some(line) = self.lines.get_mut(self.cursor_line) {
                if self.cursor_col <= line.len() {
                    line.remove(self.cursor_col - 1);
                    self.cursor_col -= 1;
                    self.is_modified = true;
                }
            }
        } else if self.cursor_line > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            let prev_line = &mut self.lines[self.cursor_line];
            let prev_len = prev_line.len();
            prev_line.push_str(&current_line);
            self.cursor_col = prev_len;
            self.is_modified = true;
        }
    }

    /// Deletes the character at the current cursor position (Delete key).
    pub fn delete_char(&mut self) {
        self.save_snapshot();
        if self.cursor_line < self.lines.len() {
            let line_len = self.lines[self.cursor_line].len();
            if self.cursor_col < line_len {
                self.lines[self.cursor_line].remove(self.cursor_col);
                self.is_modified = true;
            } else if self.cursor_line + 1 < self.lines.len() {
                // Merge next line into current
                let next_line = self.lines.remove(self.cursor_line + 1);
                self.lines[self.cursor_line].push_str(&next_line);
                self.is_modified = true;
            }
        }
    }

    /// Moves cursor upward one line.
    pub fn move_cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.clamp_cursor_col();
            if self.cursor_line < self.scroll_offset {
                self.scroll_offset = self.cursor_line;
            }
        }
    }

    /// Moves cursor downward one line.
    pub fn move_cursor_down(&mut self) {
        if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.clamp_cursor_col();
        }
    }

    /// Moves cursor left one column.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    /// Moves cursor right one column.
    pub fn move_cursor_right(&mut self) {
        let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    /// Moves cursor to the start of the current line.
    pub fn move_to_line_start(&mut self) {
        self.cursor_col = 0;
    }

    /// Moves cursor to the end of the current line.
    pub fn move_to_line_end(&mut self) {
        self.cursor_col = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
    }

    /// Scrolls the viewport up by `delta` lines.
    pub fn scroll_up(&mut self, delta: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(delta);
    }

    /// Scrolls the viewport down by `delta` lines.
    pub fn scroll_down(&mut self, delta: usize) {
        let max_scroll = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + delta).min(max_scroll);
    }

    /// Ensures the cursor is within visible vertical and horizontal viewport boundaries.
    pub fn ensure_cursor_visible(&mut self, viewport_height: usize, viewport_width: usize) {
        if viewport_height == 0 {
            return;
        }

        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        } else if self.cursor_line >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor_line.saturating_sub(viewport_height - 1);
        }

        if viewport_width > 0 {
            if self.cursor_col < self.horizontal_offset {
                self.horizontal_offset = self.cursor_col;
            } else if self.cursor_col >= self.horizontal_offset + viewport_width {
                self.horizontal_offset = self.cursor_col.saturating_sub(viewport_width - 1);
            }
        }
    }

    fn clamp_cursor_col(&mut self) {
        let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    /// Dispatches keyboard events into character insertions, deletions, newlines, and cursor movements.
    ///
    /// Supports:
    /// - Inserting typed characters (regular printable characters and shifted symbols)
    /// - `Enter`: Splits line / inserts newline
    /// - `Backspace`: Deletes preceding character or merges lines
    /// - `Delete`: Deletes current character or merges next line
    /// - Arrow keys (`Left`, `Right`, `Up`, `Down`): Cursor movement
    /// - `Home`: Moves cursor to the start of the current line
    /// - `End`: Moves cursor to the end of the current line
    /// - `PageUp` / `PageDown`: Fast vertical scrolling
    ///
    /// Returns `true` if the event was handled and mutated state.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // If search modal is active, intercept search input
        if self.is_search_open {
            match key.code {
                KeyCode::Esc => {
                    self.close_search();
                    return true;
                }
                KeyCode::Enter => {
                    self.next_search_match();
                    return true;
                }
                KeyCode::Up => {
                    self.prev_search_match();
                    return true;
                }
                KeyCode::Down => {
                    self.next_search_match();
                    return true;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.update_search_matches();
                    if let Some(&(l, c)) = self.search_matches.first() {
                        self.cursor_line = l;
                        self.cursor_col = c;
                    }
                    return true;
                }
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    self.search_query.push(c);
                    self.update_search_matches();
                    if let Some(&(l, c)) = self.search_matches.first() {
                        self.cursor_line = l;
                        self.cursor_col = c;
                    }
                    return true;
                }
                _ => return false,
            }
        }

        // Global shortcuts with CONTROL
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('z') | KeyCode::Char('Z') => {
                    return self.undo();
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    return self.redo();
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    self.open_search();
                    return true;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.insert_char(c);
                true
            }
            KeyCode::Enter => {
                self.insert_newline();
                true
            }
            KeyCode::Backspace => {
                self.delete_backspace();
                true
            }
            KeyCode::Delete => {
                self.delete_char();
                true
            }
            KeyCode::Left => {
                self.move_cursor_left();
                true
            }
            KeyCode::Right => {
                self.move_cursor_right();
                true
            }
            KeyCode::Up => {
                self.move_cursor_up();
                true
            }
            KeyCode::Down => {
                self.move_cursor_down();
                true
            }
            KeyCode::Home => {
                self.move_to_line_start();
                true
            }
            KeyCode::End => {
                self.move_to_line_end();
                true
            }
            KeyCode::PageUp => {
                self.scroll_up(8);
                true
            }
            KeyCode::PageDown => {
                self.scroll_down(8);
                true
            }
            _ => false,
        }
    }
}

/// Syntax-highlighted Code Submission Editor Ratatui widget.
#[derive(Debug, Clone, Default)]
pub struct EditorWidget<'a> {
    pub block: Option<Block<'a>>,
    pub show_line_numbers: bool,
    pub is_focused: bool,
}

impl<'a> EditorWidget<'a> {
    /// Creates a new code submission editor widget.
    pub fn new() -> Self {
        Self {
            block: None,
            show_line_numbers: true,
            is_focused: false,
        }
    }

    /// Attaches an enclosing container block.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets whether to render line numbers in the gutter.
    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Sets the focus styling of the editor.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }
}

impl<'a> StatefulWidget for EditorWidget<'a> {
    type State = EditorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner_area = match &self.block {
            Some(b) => {
                let inner = b.inner(area);
                b.clone().render(area, buf);
                inner
            }
            None => area,
        };

        if inner_area.height == 0 || inner_area.width == 0 {
            return;
        }

        state.ensure_cursor_visible(inner_area.height as usize, inner_area.width as usize);

        let gutter_width = if self.show_line_numbers {
            // Line numbers width based on total lines
            let digits = state.lines.len().max(1).to_string().len().max(2);
            digits + 3 // number + " │ "
        } else {
            0
        };

        let available_height = if state.is_search_open {
            inner_area.height.saturating_sub(1)
        } else {
            inner_area.height
        };

        let visible_lines = available_height as usize;
        let start_line = state.scroll_offset;
        let end_line = (start_line + visible_lines).min(state.lines.len());

        for (i, line_idx) in (start_line..end_line).enumerate() {
            let row = inner_area.y + i as u16;
            let is_cursor_row = self.is_focused && line_idx == state.cursor_line;

            let mut col = inner_area.x;

            // Render line number gutter
            if self.show_line_numbers {
                let num_str = format!("{:>width$} │ ", line_idx + 1, width = gutter_width - 3);
                let gutter_style = if is_cursor_row {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                for ch in num_str.chars() {
                    if col < inner_area.x + inner_area.width {
                        buf.get_mut(col, row).set_char(ch).set_style(gutter_style);
                        col += 1;
                    }
                }
            }

            // Render syntax-highlighted code line
            let raw_line = &state.lines[line_idx];
            let spans = highlight_rust_line(raw_line);

            let mut char_idx = 0;
            for span in spans {
                for ch in span.content.chars() {
                    if char_idx >= state.horizontal_offset && col < inner_area.x + inner_area.width
                    {
                        let mut style = span.style;
                        let is_match = state.is_search_open
                            && !state.search_query.is_empty()
                            && state.search_matches.iter().any(|&(l, c)| {
                                l == line_idx
                                    && char_idx >= c
                                    && char_idx < c + state.search_query.len()
                            });

                        if is_match {
                            style = Style::default()
                                .fg(Color::Black)
                                .bg(Color::Yellow)
                                .add_modifier(Modifier::BOLD);
                        } else if is_cursor_row && char_idx == state.cursor_col {
                            // Highlight cursor position with reversed colors
                            style = style.add_modifier(Modifier::REVERSED);
                        }

                        buf.get_mut(col, row).set_char(ch).set_style(style);
                        col += 1;
                    }
                    char_idx += 1;
                }
            }

            // If cursor is at the end of the line on the cursor row
            if is_cursor_row
                && state.cursor_col >= char_idx
                && col < inner_area.x + inner_area.width
            {
                buf.get_mut(col, row)
                    .set_char(' ')
                    .set_style(Style::default().add_modifier(Modifier::REVERSED));
            }
        }

        // Render search bar at bottom when search is active
        if state.is_search_open && inner_area.height > 1 {
            let search_row = inner_area.y + inner_area.height - 1;
            let match_count = state.search_matches.len();
            let match_info = if match_count == 0 {
                "0/0".to_string()
            } else {
                format!("{}/{}", state.current_match_index + 1, match_count)
            };
            let search_bar = format!(
                " Search: {} [{}] (Enter: Next, Up: Prev, Esc: Close) ",
                state.search_query, match_info
            );
            let search_style = Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);

            for (idx, ch) in search_bar.chars().enumerate() {
                let col = inner_area.x + idx as u16;
                if col < inner_area.x + inner_area.width {
                    buf.get_mut(col, search_row)
                        .set_char(ch)
                        .set_style(search_style);
                }
            }
        }
    }
}

/// Syntax highlights a line of Rust code into styled Ratatui Spans.
pub fn highlight_rust_line(line: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    // Check for comment line
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        spans.push(Span::styled(
            line.to_string(),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
        return spans;
    }

    let mut chars = line.chars().peekable();
    let mut current_token = String::new();

    while let Some(&ch) = chars.peek() {
        if ch == '/' {
            // Check for inline comment
            current_token.push(chars.next().unwrap());
            if chars.peek() == Some(&'/') {
                // Remainder of line is a comment
                for c in chars.by_ref() {
                    current_token.push(c);
                }
                spans.push(Span::styled(
                    current_token,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ));
                return spans;
            }
        } else if ch == '"' {
            // Flush token before string
            if !current_token.is_empty() {
                spans.push(classify_token(&current_token));
                current_token.clear();
            }

            // String literal
            let mut str_lit = String::new();
            str_lit.push(chars.next().unwrap());
            let mut escaped = false;

            for c in chars.by_ref() {
                str_lit.push(c);
                if c == '\\' && !escaped {
                    escaped = true;
                } else if c == '"' && !escaped {
                    break;
                } else {
                    escaped = false;
                }
            }
            spans.push(Span::styled(str_lit, Style::default().fg(Color::Green)));
        } else if ch == '\'' {
            // Flush token before char / lifetime
            if !current_token.is_empty() {
                spans.push(classify_token(&current_token));
                current_token.clear();
            }

            let mut lit = String::new();
            lit.push(chars.next().unwrap());
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' || c == '\'' {
                    lit.push(chars.next().unwrap());
                    if c == '\'' {
                        break;
                    }
                } else {
                    break;
                }
            }
            spans.push(Span::styled(lit, Style::default().fg(Color::LightCyan)));
        } else if ch.is_alphanumeric() || ch == '_' {
            current_token.push(chars.next().unwrap());
        } else if ch.is_whitespace() {
            if !current_token.is_empty() {
                spans.push(classify_token(&current_token));
                current_token.clear();
            }
            let mut ws = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    ws.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            spans.push(Span::raw(ws));
        } else {
            // Punctuation / Operator
            if !current_token.is_empty() {
                spans.push(classify_token(&current_token));
                current_token.clear();
            }
            let op = chars.next().unwrap().to_string();
            let op_style = match op.as_str() {
                "->" | "=>" | "::" => Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                "{" | "}" | "(" | ")" | "[" | "]" => Style::default().fg(Color::White),
                ";" | "," | "." => Style::default().fg(Color::DarkGray),
                _ => Style::default().fg(Color::LightYellow),
            };
            spans.push(Span::styled(op, op_style));
        }
    }

    if !current_token.is_empty() {
        spans.push(classify_token(&current_token));
    }

    spans
}

fn classify_token(token: &str) -> Span<'static> {
    // Rust keywords
    const KEYWORDS: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "true", "type", "unsafe", "use", "where", "while",
    ];

    // Standard types
    const TYPES: &[&str] = &[
        "String", "Option", "Result", "Some", "None", "Ok", "Err", "Vec", "Box", "Arc", "Rc", "u8",
        "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
        "f64", "bool", "char", "str", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
    ];

    if KEYWORDS.contains(&token) {
        Span::styled(
            token.to_string(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
    } else if TYPES.contains(&token) {
        Span::styled(
            token.to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else if token.ends_with('!') || token == "println" || token == "vec" || token == "format" {
        Span::styled(
            token.to_string(),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )
    } else if token.chars().all(|c| c.is_ascii_digit()) {
        Span::styled(token.to_string(), Style::default().fg(Color::LightCyan))
    } else {
        Span::styled(token.to_string(), Style::default().fg(Color::White))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn test_editor_handle_key_typing_and_navigation() {
        let mut state = EditorState::new("");
        assert_eq!(state.get_code(), "");

        // Typing characters
        assert!(state.handle_key(make_key(KeyCode::Char('l'))));
        assert!(state.handle_key(make_key(KeyCode::Char('e'))));
        assert!(state.handle_key(make_key(KeyCode::Char('t'))));
        assert!(state.handle_key(make_key(KeyCode::Char(' '))));
        assert!(state.handle_key(make_key(KeyCode::Char('x'))));
        assert_eq!(state.get_code(), "let x");

        // Enter for newline
        assert!(state.handle_key(make_key(KeyCode::Enter)));
        assert_eq!(state.get_code(), "let x\n");
        assert_eq!(state.cursor_line, 1);
        assert_eq!(state.cursor_col, 0);

        // Type on new line
        assert!(state.handle_key(make_key(KeyCode::Char('1'))));
        assert_eq!(state.get_code(), "let x\n1");

        // Backspace
        assert!(state.handle_key(make_key(KeyCode::Backspace)));
        assert_eq!(state.get_code(), "let x\n");

        // Backspace at line start merges lines
        assert!(state.handle_key(make_key(KeyCode::Backspace)));
        assert_eq!(state.get_code(), "let x");
        assert_eq!(state.cursor_line, 0);
        assert_eq!(state.cursor_col, 5);

        // Home and End
        assert!(state.handle_key(make_key(KeyCode::Home)));
        assert_eq!(state.cursor_col, 0);
        assert!(state.handle_key(make_key(KeyCode::End)));
        assert_eq!(state.cursor_col, 5);

        // Arrow movement
        assert!(state.handle_key(make_key(KeyCode::Left)));
        assert_eq!(state.cursor_col, 4);
        assert!(state.handle_key(make_key(KeyCode::Right)));
        assert_eq!(state.cursor_col, 5);

        // Delete key
        assert!(state.handle_key(make_key(KeyCode::Home)));
        assert!(state.handle_key(make_key(KeyCode::Delete)));
        assert_eq!(state.get_code(), "et x");
    }

    #[test]
    fn test_editor_undo_redo() {
        let mut state = EditorState::new("let a = 1;");
        state.insert_char(' ');
        state.insert_char('2');
        assert_eq!(state.get_code(), " 2let a = 1;");

        // Undo
        assert!(state.undo());
        assert_eq!(state.get_code(), " let a = 1;");
        assert!(state.undo());
        assert_eq!(state.get_code(), "let a = 1;");

        // Redo
        assert!(state.redo());
        assert_eq!(state.get_code(), " let a = 1;");
        assert!(state.redo());
        assert_eq!(state.get_code(), " 2let a = 1;");
    }

    #[test]
    fn test_editor_auto_indentation() {
        let mut state = EditorState::new("fn main() {");
        state.cursor_line = 0;
        state.cursor_col = 11;

        state.insert_newline();
        assert_eq!(state.lines.len(), 2);
        assert_eq!(state.lines[1], "    ");
        assert_eq!(state.cursor_col, 4);
    }

    #[test]
    fn test_editor_in_buffer_search() {
        let mut state = EditorState::new("let foo = 1;\nlet bar = foo + 2;\nfoo");
        state.open_search();
        assert!(state.is_search_open);

        state.search_query = "foo".to_string();
        state.update_search_matches();
        assert_eq!(state.search_matches.len(), 3);
        assert_eq!(state.search_matches[0], (0, 4));
        assert_eq!(state.search_matches[1], (1, 10));
        assert_eq!(state.search_matches[2], (2, 0));

        state.next_search_match();
        assert_eq!(state.cursor_line, 1);
        assert_eq!(state.cursor_col, 10);

        state.prev_search_match();
        assert_eq!(state.cursor_line, 0);
        assert_eq!(state.cursor_col, 4);

        state.close_search();
        assert!(!state.is_search_open);
        assert!(state.search_query.is_empty());
    }
}
