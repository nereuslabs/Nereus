//! Runtime Provider Hot-Switching Dialog Widget for Nereus TUI.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crate::widgets::centered_rect;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// Predefined LLM provider options for runtime hot-switching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderOption {
    pub id: &'static str,
    pub name: &'static str,
    pub model: &'static str,
    pub description: &'static str,
    pub is_offline: bool,
}

pub const AVAILABLE_PROVIDERS: &[ProviderOption] = &[
    ProviderOption {
        id: "stub",
        name: "StubLLM (Offline Deterministic)",
        model: "deterministic-mock-v1",
        description: "Zero token cost, instant offline pedagogical evaluation",
        is_offline: true,
    },
    ProviderOption {
        id: "openrouter",
        name: "OpenRouter (Claude 3.5 Sonnet)",
        model: "anthropic/claude-3.5-sonnet",
        description: "High-density Socratic reasoning via OpenRouter API",
        is_offline: false,
    },
    ProviderOption {
        id: "anthropic",
        name: "Anthropic Direct (Claude 3.5 Sonnet)",
        model: "claude-3-5-sonnet-20241022",
        description: "Direct official Anthropic API with streaming support",
        is_offline: false,
    },
    ProviderOption {
        id: "ollama",
        name: "Local Ollama (Llama 3.3 / Qwen)",
        model: "http://localhost:11434/v1",
        description: "Private on-device local inference without external telemetry",
        is_offline: true,
    },
];

/// State tracker for the provider selection dialog.
#[derive(Debug, Clone)]
pub struct ProviderState {
    pub selected_index: usize,
    pub active_provider_id: String,
}

impl Default for ProviderState {
    fn default() -> Self {
        Self {
            selected_index: 0,
            active_provider_id: "stub".to_string(),
        }
    }
}

impl ProviderState {
    pub fn new(active_id: &str) -> Self {
        let index = AVAILABLE_PROVIDERS
            .iter()
            .position(|p| p.id == active_id)
            .unwrap_or(0);
        Self {
            selected_index: index,
            active_provider_id: active_id.to_string(),
        }
    }

    pub fn select_next(&mut self) {
        if !AVAILABLE_PROVIDERS.is_empty() {
            self.selected_index = (self.selected_index + 1) % AVAILABLE_PROVIDERS.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !AVAILABLE_PROVIDERS.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = AVAILABLE_PROVIDERS.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Handles dialog keyboard events. Returns `Some(provider_id)` when confirmed with Enter.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<&'static str> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                None
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let digit = c.to_digit(10).unwrap_or(0) as usize;
                if digit >= 1 && digit <= AVAILABLE_PROVIDERS.len() {
                    self.selected_index = digit - 1;
                }
                None
            }
            KeyCode::Enter => {
                let selected = &AVAILABLE_PROVIDERS[self.selected_index];
                self.active_provider_id = selected.id.to_string();
                Some(selected.id)
            }
            _ => None,
        }
    }
}

/// Floating modal dialog widget for runtime provider hot-switching.
pub struct ProviderDialogWidget<'a> {
    pub state: &'a ProviderState,
}

impl<'a> ProviderDialogWidget<'a> {
    pub fn new(state: &'a ProviderState) -> Self {
        Self { state }
    }

    pub fn render(self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(64, 52, area);
        f.render_widget(Clear, popup_area);

        let border_block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " ⚡ Runtime Provider Hot-Switching (Tier-1) ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Rgb(15, 23, 42)));

        f.render_widget(border_block, popup_area);

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2), // Subtitle
                Constraint::Min(4),    // Options list
                Constraint::Length(2), // Navigation hint
            ])
            .split(popup_area);

        // Subtitle
        let subtitle = Paragraph::new(vec![Line::from(vec![
            Span::styled("Active Provider: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &self.state.active_provider_id,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (Hot-swaps model & inference client instantly)",
                Style::default().fg(Color::Gray),
            ),
        ])])
        .alignment(Alignment::Left);
        f.render_widget(subtitle, inner_layout[0]);

        // Provider Options
        let mut option_lines = Vec::new();
        for (idx, provider) in AVAILABLE_PROVIDERS.iter().enumerate() {
            let is_selected = idx == self.state.selected_index;
            let is_active = provider.id == self.state.active_provider_id;

            let cursor = if is_selected { " ▶ " } else { "   " };
            let active_badge = if is_active { " [ACTIVE]" } else { "" };

            let title_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(30, 58, 138))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::LightCyan)
            };

            let badge_style = if is_active {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            option_lines.push(Line::from(vec![
                Span::styled(
                    cursor,
                    if is_selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(
                    format!("{}. ", idx + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(provider.name, title_style),
                Span::styled(active_badge, badge_style),
            ]));

            let desc_indent = "      ";
            option_lines.push(Line::from(vec![
                Span::raw(desc_indent),
                Span::styled(
                    format!("Model: {} • {}", provider.model, provider.description),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            option_lines.push(Line::raw(""));
        }

        let options_paragraph = Paragraph::new(option_lines);
        f.render_widget(options_paragraph, inner_layout[1]);

        // Footer hints
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" ▲/▼/j/k", Style::default().fg(Color::Yellow)),
            Span::styled(" Navigate  •  ", Style::default().fg(Color::DarkGray)),
            Span::styled("1-4", Style::default().fg(Color::Yellow)),
            Span::styled(" Quick Pick  •  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Switch Provider  •  ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("Esc/q", Style::default().fg(Color::Red)),
            Span::styled(" Cancel", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Center);
        f.render_widget(footer, inner_layout[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn test_provider_state_navigation() {
        let mut state = ProviderState::new("stub");
        assert_eq!(state.selected_index, 0);

        state.select_next();
        assert_eq!(state.selected_index, 1);

        state.select_prev();
        assert_eq!(state.selected_index, 0);

        // Wrap around to last
        state.select_prev();
        assert_eq!(state.selected_index, AVAILABLE_PROVIDERS.len() - 1);
    }

    #[test]
    fn test_provider_state_keys() {
        let mut state = ProviderState::new("stub");

        // Key down
        assert_eq!(state.handle_key(make_key(KeyCode::Down)), None);
        assert_eq!(state.selected_index, 1);

        // Key digit '3' (Anthropic)
        assert_eq!(state.handle_key(make_key(KeyCode::Char('3'))), None);
        assert_eq!(state.selected_index, 2);

        // Enter selects provider
        let result = state.handle_key(make_key(KeyCode::Enter));
        assert_eq!(result, Some("anthropic"));
        assert_eq!(state.active_provider_id, "anthropic");
    }
}
