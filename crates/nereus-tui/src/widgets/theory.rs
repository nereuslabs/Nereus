//! Theory and challenge viewer widget with markdown styling, code syntax highlighting, and scroll offset tracking.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crate::widgets::editor::highlight_rust_line;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Widget};

/// Theory & Challenge viewer widget with styled headings and code syntax highlighting.
#[derive(Debug, Clone)]
pub struct TheoryViewerWidget<'a> {
    pub content: &'a str,
    pub scroll_offset: usize,
    pub block: Option<Block<'a>>,
    pub is_focused: bool,
}

impl<'a> TheoryViewerWidget<'a> {
    /// Creates a new theory viewer with the given markdown/text content and scroll offset.
    pub fn new(content: &'a str, scroll_offset: usize) -> Self {
        Self {
            content,
            scroll_offset,
            block: None,
            is_focused: false,
        }
    }

    /// Attaches an enclosing container block.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the focus styling of the widget.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Parses markdown-like content into styled lines, applying Rust syntax highlighting within code fences.
    pub fn parse_styled_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut in_code_fence = false;

        for raw_line in self.content.lines() {
            let trimmed = raw_line.trim();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                let fence_style = Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                lines.push(Line::from(vec![Span::styled(
                    raw_line.to_string(),
                    fence_style,
                )]));
                continue;
            }

            if in_code_fence {
                // Apply syntax highlighting to code inside code block
                let code_spans = highlight_rust_line(raw_line);
                lines.push(Line::from(code_spans));
            } else if trimmed.starts_with("# ") {
                // H1 Heading
                lines.push(Line::from(vec![Span::styled(
                    raw_line.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )]));
            } else if trimmed.starts_with("## ") {
                // H2 Heading
                lines.push(Line::from(vec![Span::styled(
                    raw_line.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));
            } else if trimmed.starts_with("### ") {
                // H3 Heading
                lines.push(Line::from(vec![Span::styled(
                    raw_line.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )]));
            } else if trimmed.starts_with("---") || trimmed.starts_with("===") {
                // Divider line
                lines.push(Line::from(vec![Span::styled(
                    "─".repeat(48),
                    Style::default().fg(Color::DarkGray),
                )]));
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                // Bullet item
                let mut spans = vec![Span::styled(
                    " • ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )];
                spans.push(Span::styled(
                    trimmed[2..].to_string(),
                    Style::default().fg(Color::White),
                ));
                lines.push(Line::from(spans));
            } else if trimmed.starts_with('>') {
                // Blockquote / Socratic callout
                lines.push(Line::from(vec![Span::styled(
                    raw_line.to_string(),
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::ITALIC),
                )]));
            } else {
                // Regular prose line
                lines.push(Line::from(vec![Span::styled(
                    raw_line.to_string(),
                    Style::default().fg(Color::Reset),
                )]));
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(vec![Span::raw("")]));
        }

        lines
    }
}

impl<'a> Widget for TheoryViewerWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
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

        let lines = self.parse_styled_lines();
        let max_scroll = lines.len().saturating_sub(inner_area.height as usize);
        let start_line = self.scroll_offset.min(max_scroll);

        let visible_lines = lines
            .iter()
            .skip(start_line)
            .take(inner_area.height as usize);

        for (row_idx, line) in visible_lines.enumerate() {
            let row = inner_area.y + row_idx as u16;
            let mut col = inner_area.x;

            for span in &line.spans {
                for ch in span.content.chars() {
                    if col < inner_area.x + inner_area.width {
                        buf.get_mut(col, row).set_char(ch).set_style(span.style);
                        col += 1;
                    }
                }
            }
        }
    }
}
