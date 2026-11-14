//! Persistent bottom status bar widget for the Nereus TUI cockpit.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

/// Status bar widget rendering hotkey prompts and system status at the bottom of the screen.
#[derive(Debug, Clone, Default)]
pub struct StatusBarWidget<'a> {
    pub status_message: Option<&'a str>,
    pub current_state: Option<&'a str>,
}

impl<'a> StatusBarWidget<'a> {
    /// Creates a fresh status bar widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an ephemeral status message to display.
    pub fn status_message(mut self, msg: Option<&'a str>) -> Self {
        self.status_message = msg;
        self
    }

    /// Sets the active state name to display.
    pub fn current_state(mut self, state: Option<&'a str>) -> Self {
        self.current_state = state;
        self
    }
}

impl<'a> Widget for StatusBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let key_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(Color::Gray);
        let bar_style = Style::default().bg(Color::Rgb(20, 24, 32));

        // Fill background of the status line
        for x in area.left()..area.right() {
            buf.get_mut(x, area.top()).set_style(bar_style);
        }

        // Persistent bottom hotkey prompts: [Tab] Switch Pane  [Enter] Submit  [?] Help  [q] Quit
        let mut spans = vec![
            Span::raw(" "),
            Span::styled("[Tab]", key_style),
            Span::styled(" Switch Pane  ", desc_style),
            Span::styled("[Enter]", key_style),
            Span::styled(" Submit  ", desc_style),
            Span::styled("[?]", key_style),
            Span::styled(" Help  ", desc_style),
            Span::styled("[q]", key_style),
            Span::styled(" Quit", desc_style),
        ];

        if let Some(msg) = self.status_message {
            spans.push(Span::styled("  │  ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(msg, Style::default().fg(Color::Yellow)));
        } else if let Some(state) = self.current_state {
            spans.push(Span::styled(
                "  │  State: ",
                Style::default().fg(Color::DarkGray),
            ));
            spans.push(Span::styled(state, Style::default().fg(Color::LightCyan)));
        }

        let line = Line::from(spans);
        buf.set_line(area.left(), area.top(), &line, area.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_status_bar_rendering() {
        let backend = TestBackend::new(100, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let widget = StatusBarWidget::new()
                    .status_message(Some("Ready"))
                    .current_state(Some("Tutoring"));
                f.render_widget(widget, f.size());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(text.contains("[Tab]"));
        assert!(text.contains("Switch Pane"));
        assert!(text.contains("[Enter]"));
        assert!(text.contains("Submit"));
        assert!(text.contains("[?]"));
        assert!(text.contains("Help"));
        assert!(text.contains("[q]"));
        assert!(text.contains("Quit"));
        assert!(text.contains("Ready"));
    }
}
