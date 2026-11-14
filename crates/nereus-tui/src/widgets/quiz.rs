//! Diagnostic quiz widget rendering multiple-choice questions with progress bar and cursor.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crossterm::event::{KeyCode, KeyEvent};
use nereus_core::models::DiagnosticQuestion;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};

/// Interactive state tracking multiple-choice option selection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuizState {
    /// Currently highlighted option index (0-based)
    pub selected_index: usize,
    /// Whether the quiz widget currently has active focus
    pub is_focused: bool,
}

impl QuizState {
    /// Creates a fresh quiz state with the first option selected.
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects the previous option, wrapping to the last option if at the top.
    pub fn select_prev(&mut self, options_len: usize) {
        if options_len == 0 {
            self.selected_index = 0;
            return;
        }
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = options_len - 1;
        }
    }

    /// Selects the next option, wrapping to the first option if at the bottom.
    pub fn select_next(&mut self, options_len: usize) {
        if options_len == 0 {
            self.selected_index = 0;
            return;
        }
        if self.selected_index + 1 < options_len {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
    }

    /// Selects an option directly by index if within bounds.
    pub fn select(&mut self, index: usize, options_len: usize) {
        if index < options_len {
            self.selected_index = index;
        }
    }

    /// Handles keyboard events for the diagnostic quiz.
    ///
    /// Key mappings:
    /// - `1`..=`4`: Directly selects option `0..=3` and returns `Some(index)` to confirm choice.
    /// - `Up` / `k`: Moves selection cursor up, returns `None`.
    /// - `Down` / `j`: Moves selection cursor down, returns `None`.
    /// - `Enter`: Confirms current cursor selection and returns `Some(selected_index)`.
    pub fn handle_key(&mut self, key: KeyEvent, options_len: usize) -> Option<usize> {
        match key.code {
            KeyCode::Char(c @ '1'..='4') => {
                let idx = (c as usize) - ('1' as usize);
                if idx < options_len {
                    self.selected_index = idx;
                    Some(idx)
                } else {
                    None
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev(options_len);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next(options_len);
                None
            }
            KeyCode::Enter => {
                if self.selected_index < options_len {
                    Some(self.selected_index)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Multiple-choice diagnostic quiz widget with progress bar and selection cursor.
#[derive(Debug, Clone)]
pub struct DiagnosticQuizWidget<'a> {
    pub question: &'a DiagnosticQuestion,
    pub current_index: usize,
    pub total_questions: usize,
    pub block: Option<Block<'a>>,
    pub is_focused: bool,
}

impl<'a> DiagnosticQuizWidget<'a> {
    /// Creates a new diagnostic quiz widget.
    pub fn new(
        question: &'a DiagnosticQuestion,
        current_index: usize,
        total_questions: usize,
    ) -> Self {
        Self {
            question,
            current_index,
            total_questions,
            block: None,
            is_focused: false,
        }
    }

    /// Attaches an enclosing container block.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets whether the widget is actively focused.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }
}

impl<'a> StatefulWidget for DiagnosticQuizWidget<'a> {
    type State = QuizState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = match &self.block {
            Some(b) => {
                let inner = b.inner(area);
                b.clone().render(area, buf);
                inner
            }
            None => area,
        };

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        // 1. Progress Bar: "Question X of Y" + visual gauge
        let question_num = self.current_index + 1;
        let total = self.total_questions.max(1);
        let pct = (question_num * 100) / total;

        let total_bar_slots = (inner.width as usize).saturating_sub(28).clamp(8, 24);
        let filled_slots = (total_bar_slots * question_num) / total;
        let empty_slots = total_bar_slots.saturating_sub(filled_slots);
        let progress_bar = format!("[{}{}]", "█".repeat(filled_slots), "░".repeat(empty_slots));

        lines.push(Line::from(vec![
            Span::styled(
                format!("📊 Question {question_num} of {total} "),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                progress_bar,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {pct}%"),
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));

        // 2. Question Text
        lines.push(Line::from(vec![
            Span::styled(
                "❓ ",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &self.question.question,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(""));

        // 3. Multiple Choice Options with Selection Cursor
        for (idx, opt) in self.question.options.iter().enumerate() {
            let is_selected = state.selected_index == idx;
            let key_label = format!("[{}] ", idx + 1);

            if is_selected {
                lines.push(Line::from(vec![
                    Span::styled(
                        " ► ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        key_label,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        opt,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(key_label, Style::default().fg(Color::DarkGray)),
                    Span::styled(opt, Style::default().fg(Color::Gray)),
                ]));
            }
        }

        lines.push(Line::from(""));

        // 4. Instructions / Hotkeys
        lines.push(Line::from(vec![
            Span::styled(
                "💡 Options: ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "[1-4]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Choose  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "[↑/↓]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Move  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Submit", Style::default().fg(Color::DarkGray)),
        ]));

        // Render formatted lines into buffer
        for (row_offset, line) in lines.iter().enumerate() {
            if (row_offset as u16) < inner.height {
                buf.set_line(inner.x, inner.y + row_offset as u16, line, inner.width);
            }
        }
    }
}

impl<'a> Widget for DiagnosticQuizWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut default_state = QuizState::default();
        StatefulWidget::render(self, area, buf, &mut default_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn test_quiz_state_navigation() {
        let mut state = QuizState::new();
        assert_eq!(state.selected_index, 0);

        // Move down across 3 options
        state.select_next(3);
        assert_eq!(state.selected_index, 1);
        state.select_next(3);
        assert_eq!(state.selected_index, 2);
        // Wrap around to top
        state.select_next(3);
        assert_eq!(state.selected_index, 0);

        // Wrap around to bottom
        state.select_prev(3);
        assert_eq!(state.selected_index, 2);
        state.select_prev(3);
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_quiz_state_handle_key_number_keys() {
        let mut state = QuizState::new();

        // Key '1' selects option 0
        assert_eq!(state.handle_key(make_key(KeyCode::Char('1')), 4), Some(0));
        assert_eq!(state.selected_index, 0);

        // Key '3' selects option 2
        assert_eq!(state.handle_key(make_key(KeyCode::Char('3')), 4), Some(2));
        assert_eq!(state.selected_index, 2);

        // Key '4' selects option 3
        assert_eq!(state.handle_key(make_key(KeyCode::Char('4')), 4), Some(3));
        assert_eq!(state.selected_index, 3);

        // Key '4' out of bounds for 2 options returns None
        assert_eq!(state.handle_key(make_key(KeyCode::Char('4')), 2), None);
    }

    #[test]
    fn test_quiz_state_handle_key_arrows_and_enter() {
        let mut state = QuizState::new();

        // Down arrow navigates without submitting
        assert_eq!(state.handle_key(make_key(KeyCode::Down), 3), None);
        assert_eq!(state.selected_index, 1);

        // Up arrow navigates without submitting
        assert_eq!(state.handle_key(make_key(KeyCode::Up), 3), None);
        assert_eq!(state.selected_index, 0);

        // Enter submits current selection
        assert_eq!(state.handle_key(make_key(KeyCode::Enter), 3), Some(0));
    }

    #[test]
    fn test_quiz_widget_rendering() {
        let q = DiagnosticQuestion {
            id: "diag-test".to_string(),
            question: "Which trait bounds asynchronous stream items?".to_string(),
            options: vec![
                "Stream".to_string(),
                "Future".to_string(),
                "Send".to_string(),
            ],
            correct_index: Some(0),
        };

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = QuizState::new();
        state.selected_index = 1; // "Future" selected

        terminal
            .draw(|f| {
                let widget = DiagnosticQuizWidget::new(&q, 0, 3);
                f.render_stateful_widget(widget, f.size(), &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(text.contains("Question 1 of 3"));
        assert!(text.contains("Which trait bounds asynchronous stream items?"));
        assert!(text.contains("[1] Stream"));
        assert!(text.contains("[2] Future"));
        assert!(text.contains("[3] Send"));
        assert!(text.contains("► [2] Future"));
    }
}
