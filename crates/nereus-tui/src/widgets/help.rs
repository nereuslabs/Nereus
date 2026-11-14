//! Help modal overlay widget displaying hotkey cheatsheet and Socratic learning tips.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

/// Computes a centered rectangle for a modal overlay given percentage dimensions.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Centered modal overlay displaying hotkey shortcuts and Socratic mentoring guidance.
#[derive(Debug, Clone, Default)]
pub struct HelpWidget<'a> {
    pub block: Option<Block<'a>>,
}

impl<'a> HelpWidget<'a> {
    /// Creates a fresh help modal overlay widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches an enclosing container block.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a> Widget for HelpWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // Clear underlying cockpit area beneath the modal
        Clear.render(area, buf);

        let default_block = Block::default()
            .title(Line::from(vec![Span::styled(
                " 📖 Nereus Cockpit Help & Cheatsheet ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]))
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let block = self.block.unwrap_or(default_block);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let key_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::White);
        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        let socratic_style = Style::default().fg(Color::LightGreen);
        let tip_style = Style::default().fg(Color::DarkGray);

        let lines = vec![
            Line::from(Span::styled("⌨️  NAVIGATION & HOTKEYS", header_style)),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [Tab] / [BackTab] ", key_style),
                Span::styled(
                    "Cycle focus clockwise / counter-clockwise across panes",
                    desc_style,
                ),
            ]),
            Line::from(vec![
                Span::styled("  [1] - [4]         ", key_style),
                Span::styled(
                    "Quick-select and submit answer in Diagnostic Quiz",
                    desc_style,
                ),
            ]),
            Line::from(vec![
                Span::styled("  [↑] / [↓], [k]/[j]", key_style),
                Span::styled(
                    "Navigate options / scroll content / move editor cursor",
                    desc_style,
                ),
            ]),
            Line::from(vec![
                Span::styled("  [←] / [→], [h]/[l]", key_style),
                Span::styled("Move editor cursor left / right", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  [Enter]           ", key_style),
                Span::styled(
                    "Submit challenge solution / confirm quiz choice",
                    desc_style,
                ),
            ]),
            Line::from(vec![
                Span::styled("  [Ctrl+Enter]      ", key_style),
                Span::styled(
                    "Submit code solution directly from editor buffer",
                    desc_style,
                ),
            ]),
            Line::from(vec![
                Span::styled("  [?]               ", key_style),
                Span::styled("Toggle this help modal overlay", desc_style),
            ]),
            Line::from(vec![
                Span::styled("  [q] / [Esc]       ", key_style),
                Span::styled("Quit cockpit / dismiss help overlay", desc_style),
            ]),
            Line::from(""),
            Line::from(Span::styled("🦉 SOCRATIC LEARNING TIPS", header_style)),
            Line::from(""),
            Line::from(Span::styled(
                "  • Nereus does not spoon-feed code — the cognitive work is yours.",
                socratic_style,
            )),
            Line::from(Span::styled(
                "  • The Lighthouse Beacon reflects compiler invariants and memory semantics.",
                socratic_style,
            )),
            Line::from(Span::styled(
                "  • Review the Theory pane carefully before formulating code solutions.",
                socratic_style,
            )),
            Line::from(Span::styled(
                "  • When compiler errors occur, reflect on borrow lifetimes and ownership.",
                socratic_style,
            )),
            Line::from(Span::styled(
                "  • Topics unlock progressively as sandbox test assertions pass.",
                socratic_style,
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press [?] or [Esc] to dismiss this overlay and return to the cockpit.",
                tip_style,
            )),
        ];

        let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
        paragraph.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_help_widget_rendering() {
        let backend = TestBackend::new(80, 26);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = HelpWidget::new();
                f.render_widget(widget, f.size());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(text.contains("Nereus Cockpit Help & Cheatsheet"));
        assert!(text.contains("NAVIGATION & HOTKEYS"));
        assert!(text.contains("[Tab]"));
        assert!(text.contains("[1] - [4]"));
        assert!(text.contains("[Enter]"));
        assert!(text.contains("[?]"));
        assert!(text.contains("SOCRATIC LEARNING TIPS"));
    }

    #[test]
    fn test_centered_rect_calculation() {
        let total = Rect::new(0, 0, 100, 100);
        let popup = centered_rect(60, 60, total);

        assert_eq!(popup.width, 60);
        assert_eq!(popup.height, 60);
        assert_eq!(popup.x, 20);
        assert_eq!(popup.y, 20);
    }
}
