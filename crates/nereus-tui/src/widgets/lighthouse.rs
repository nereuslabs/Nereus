//! Animated Socratic Lighthouse Widget.
//!
//! Renders a multi-frame retro pixel-art ASCII lighthouse with a 360-degree rotating
//! "Beam of Clarity", illuminated lantern room, and animated oceanic baseline waves.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Widget};

/// Total distinct rotation frames representing the 360° sweep of clarity.
pub const TOTAL_LIGHTHOUSE_FRAMES: usize = 8;

/// Retro pixel-art Socratic Lighthouse widget with dynamic rotating beam animation.
#[derive(Debug, Clone)]
pub struct LighthouseWidget<'a> {
    pub tick: u64,
    pub status: Option<&'a str>,
    pub block: Option<Block<'a>>,
}

impl<'a> LighthouseWidget<'a> {
    /// Creates a new lighthouse widget with the given tick counter.
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            status: None,
            block: None,
        }
    }

    /// Sets optional status or motto to display beneath the beacon.
    pub fn with_status(mut self, status: &'a str) -> Self {
        self.status = Some(status);
        self
    }

    /// Attaches an enclosing Ratatui Block with borders and title.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Returns the current rotation frame index in range `0..8`.
    pub fn frame_index(&self) -> usize {
        (self.tick as usize) % TOTAL_LIGHTHOUSE_FRAMES
    }

    /// Returns human-readable directional description of the rotating beam.
    pub fn beam_direction(&self) -> &'static str {
        match self.frame_index() {
            0 => "East (90° - Clear Path)",
            1 => "South-East (135° - Horizon Sweep)",
            2 => "South (180° - Direct Socratic Illumination)",
            3 => "South-West (225° - Deep Reflection)",
            4 => "West (270° - Guiding Light)",
            5 => "North-West (315° - Horizon Return)",
            6 => "North (0° - Zenith Beam)",
            7 => "North-East (45° - Ascending Clarity)",
            _ => "Illuminating",
        }
    }

    /// Generates styled text lines representing the animated lighthouse for the current frame.
    pub fn render_lines(&self) -> Vec<Line<'static>> {
        let frame = self.frame_index();
        let mut lines = Vec::new();

        // 1. Lantern cap / Spire and top beam sweep
        match frame {
            6 => {
                // Zenith beam shooting straight upward
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled(
                        "▲ ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "(Zenith Clarity)",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled(
                        "║",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            7 => {
                // Diagonal up-right
                lines.push(Line::from(vec![
                    Span::raw("                        "),
                    Span::styled(
                        "◥ ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("(Clarity)", Style::default().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("                      "),
                    Span::styled("╱", Style::default().fg(Color::Yellow)),
                ]));
            }
            5 => {
                // Diagonal up-left
                lines.push(Line::from(vec![
                    Span::raw("(Clarity) "),
                    Span::styled(
                        "◤",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("           "),
                    Span::styled("╲", Style::default().fg(Color::Yellow)),
                ]));
            }
            _ => {
                // Normal top spacing
                lines.push(Line::from(vec![Span::raw("")]));
            }
        }

        // Spire
        lines.push(Line::from(vec![
            Span::raw("                 "),
            Span::styled(
                "▄█▄",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // 2. Lantern room & rotating horizontal / diagonal beam
        let lantern_core = match frame {
            0 | 4 => "[***]",
            1 | 3 => "[*✦*]",
            2 => "[★*★]",
            6 => "[•*•]",
            _ => "[·*·]",
        };

        let lantern_span = Span::styled(
            lantern_core,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        match frame {
            0 => {
                // Beam pointing East (Right)
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    lantern_span,
                    Span::styled(
                        "════════════════════════════► ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "(Beam of Clarity)",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            1 => {
                // Beam pointing South-East (Down-Right)
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    lantern_span,
                    Span::styled(
                        " ╲",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            2 => {
                // Beam pointing South / Facing Viewer (Direct illumination)
                lines.push(Line::from(vec![
                    Span::raw("       "),
                    Span::styled(
                        "░▒▓[ ★ S O C R A T I C   C L A R I T Y ★ ]▓▒░",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            3 => {
                // Beam pointing South-West (Down-Left)
                lines.push(Line::from(vec![
                    Span::raw("              "),
                    Span::styled(
                        "╱ ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    lantern_span,
                ]));
            }
            4 => {
                // Beam pointing West (Left)
                lines.push(Line::from(vec![
                    Span::styled(
                        "(Beam of Clarity) ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "◄════════════════════════",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    lantern_span,
                ]));
            }
            _ => {
                // Other frames
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    lantern_span,
                ]));
            }
        }

        // 3. Lighthouse tower body with alternating bands and diagonal beams
        match frame {
            1 => {
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled("█████", Style::default().fg(Color::Red)),
                    Span::styled("   ╲", Style::default().fg(Color::Yellow)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled("░░░░░", Style::default().fg(Color::White)),
                    Span::styled("     ╲", Style::default().fg(Color::Yellow)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled("█████", Style::default().fg(Color::Red)),
                    Span::styled("       ╲", Style::default().fg(Color::Yellow)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("               "),
                    Span::styled("░░░░░░░", Style::default().fg(Color::White)),
                    Span::styled(
                        "        ▼ ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("(Beam of Clarity)", Style::default().fg(Color::Cyan)),
                ]));
            }
            3 => {
                lines.push(Line::from(vec![
                    Span::raw("            "),
                    Span::styled("╱   ", Style::default().fg(Color::Yellow)),
                    Span::styled("█████", Style::default().fg(Color::Red)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("          "),
                    Span::styled("╱     ", Style::default().fg(Color::Yellow)),
                    Span::styled("░░░░░", Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("        "),
                    Span::styled("╱       ", Style::default().fg(Color::Yellow)),
                    Span::styled("█████", Style::default().fg(Color::Red)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("(Clarity) ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        "◄       ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("░░░░░░░", Style::default().fg(Color::White)),
                ]));
            }
            _ => {
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled("█████", Style::default().fg(Color::Red)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled("░░░░░", Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled("█████", Style::default().fg(Color::Red)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("               "),
                    Span::styled("░░░░░░░", Style::default().fg(Color::White)),
                ]));
            }
        }

        // 4. Base masonry foundation
        lines.push(Line::from(vec![
            Span::raw("              "),
            Span::styled("█████████", Style::default().fg(Color::Red)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("             "),
            Span::styled("▓▓▓▓▓▓▓▓▓▓▓", Style::default().fg(Color::DarkGray)),
        ]));

        // 5. Ocean baseline with animated phase waves
        let wave_phase = self.tick.is_multiple_of(2);
        let wave_line_1 = if wave_phase {
            "         ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
        } else {
            "        ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ "
        };
        let wave_line_2 = if wave_phase {
            "        ≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋"
        } else {
            "       ≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋ "
        };

        lines.push(Line::from(vec![Span::styled(
            wave_line_1,
            Style::default().fg(Color::Blue),
        )]));
        lines.push(Line::from(vec![Span::styled(
            wave_line_2,
            Style::default().fg(Color::Cyan),
        )]));

        // 6. Optional status / directional indicator
        if let Some(status_text) = self.status {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("🧭 Beacon: {} | {}", self.beam_direction(), status_text),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        lines
    }
}

impl<'a> Widget for LighthouseWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl<'a> Widget for &LighthouseWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = self.render_lines();
        let mut paragraph = Paragraph::new(lines).alignment(Alignment::Left);

        if let Some(block) = &self.block {
            paragraph = paragraph.block(block.clone());
        }

        paragraph.render(area, buf);
    }
}
