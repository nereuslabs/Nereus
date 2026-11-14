//! Roadmap progress and curriculum viewer with scroll offset tracking and topic selection.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use nereus_core::models::RoadmapTopic;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};

/// Reactive state tracking roadmap selection and viewport scroll offset.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoadmapState {
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub is_focused: bool,
}

impl RoadmapState {
    /// Creates a new default roadmap navigation state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the selection cursor to the next topic.
    pub fn select_next(&mut self, total_items: usize) {
        if total_items > 0 && self.selected_index + 1 < total_items {
            self.selected_index += 1;
        }
    }

    /// Moves the selection cursor to the previous topic.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Scrolls the viewport up by `delta` lines.
    pub fn scroll_up(&mut self, delta: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(delta);
    }

    /// Scrolls the viewport down by `delta` lines.
    pub fn scroll_down(&mut self, delta: usize, total_lines: usize, viewport_height: usize) {
        if total_lines > viewport_height {
            let max_offset = total_lines.saturating_sub(viewport_height);
            self.scroll_offset = (self.scroll_offset + delta).min(max_offset);
        }
    }

    /// Adjusts viewport scroll so the currently selected topic remains in view.
    pub fn ensure_selected_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        // Each topic item takes roughly 2-3 lines (title + spacing/description)
        let item_approx_line = self.selected_index * 2;
        if item_approx_line < self.scroll_offset {
            self.scroll_offset = item_approx_line;
        } else if item_approx_line >= self.scroll_offset + viewport_height {
            self.scroll_offset = item_approx_line.saturating_sub(viewport_height - 1);
        }
    }
}

/// Interactive Roadmap visualization widget with progress tracking.
#[derive(Debug, Clone)]
pub struct RoadmapWidget<'a> {
    pub topics: &'a [RoadmapTopic],
    pub current_topic_index: usize,
    pub block: Option<Block<'a>>,
    pub is_focused: bool,
}

impl<'a> RoadmapWidget<'a> {
    /// Creates a new RoadmapWidget from a slice of topics and the active topic pointer.
    pub fn new(topics: &'a [RoadmapTopic], current_topic_index: usize) -> Self {
        Self {
            topics,
            current_topic_index,
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

    /// Helper calculating mastery percentage (0..=100).
    pub fn completion_percentage(&self) -> u32 {
        if self.topics.is_empty() {
            0
        } else {
            let completed = self.current_topic_index.min(self.topics.len());
            ((completed as f32 / self.topics.len() as f32) * 100.0).round() as u32
        }
    }
}

impl<'a> StatefulWidget for RoadmapWidget<'a> {
    type State = RoadmapState;

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

        let mut lines = Vec::new();

        // 1. Progress Bar Header
        let total = self.topics.len();
        let completed = self.current_topic_index.min(total);
        let pct = self.completion_percentage();

        let bar_width = 16.min(inner_area.width.saturating_sub(20) as usize);
        let filled_chars = (completed * bar_width).checked_div(total).unwrap_or(0);
        let unfilled_chars = bar_width.saturating_sub(filled_chars);

        let progress_spans = vec![
            Span::raw("Mastery: ["),
            Span::styled(
                "█".repeat(filled_chars),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "░".repeat(unfilled_chars),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(format!("] {pct}% ({completed}/{total})")),
        ];
        lines.push(Line::from(progress_spans));
        lines.push(Line::from(vec![Span::raw("")]));

        // 2. Render Topics
        if self.topics.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  [~] ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Synthesizing personalized learning roadmap...",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            for (i, topic) in self.topics.iter().enumerate() {
                let is_selected = self.is_focused && i == state.selected_index;

                // Status prefix badge
                let status_prefix = if i < self.current_topic_index {
                    Span::styled(
                        "[✔] ",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if i == self.current_topic_index {
                    Span::styled(
                        "[●] ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
                };

                let pointer = if is_selected {
                    Span::styled(
                        "► ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("  ")
                };

                let title_style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if i == self.current_topic_index {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if i < self.current_topic_index {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let title_span = Span::styled(format!("{}. {}", i + 1, topic.title), title_style);

                let duration_span = if topic.estimated_hours > 0.0 {
                    Span::styled(
                        format!(" (~{:.1}h)", topic.estimated_hours),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("")
                };

                lines.push(Line::from(vec![
                    pointer,
                    status_prefix,
                    title_span,
                    duration_span,
                ]));

                // For the selected topic or current topic, show description if space permits
                if (is_selected || (i == self.current_topic_index && !self.is_focused))
                    && !topic.description.is_empty()
                {
                    let desc_text = format!("      ↳ {}", topic.description);
                    lines.push(Line::from(vec![Span::styled(
                        desc_text,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    )]));
                }
            }
        }

        // 3. Render lines with scroll offset
        state.ensure_selected_visible(inner_area.height as usize);
        let start_line = state.scroll_offset.min(lines.len().saturating_sub(1));
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
