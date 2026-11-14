//! Terminal Cockpit layout manager.
//!
//! Calculates spatial split-pane layouts for the Nereus Ratatui interface.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Spatial bounding boxes for the 4 primary cockpit panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CockpitPanes {
    /// Top-left pane: Roadmap and mastery progress
    pub roadmap: Rect,
    /// Top-right pane: Challenge theory and instructions
    pub task: Rect,
    /// Bottom-left pane: Socratic guidance and animated lighthouse beacon
    pub socratic_guidance: Rect,
    /// Bottom-right pane: Student code submission editor
    pub code_editor: Rect,
}

/// Computes the 4-pane split layout for a given terminal bounding area.
///
/// Layout distribution:
/// - Vertical split: 55% top, 45% bottom
/// - Top horizontal split: 40% Roadmap, 60% Challenge & Theory
/// - Bottom horizontal split: 40% Socratic Guidance, 60% Code Submission
pub fn calculate_cockpit_layout(area: Rect) -> CockpitPanes {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_chunks[0]);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_chunks[1]);

    CockpitPanes {
        roadmap: top_chunks[0],
        task: top_chunks[1],
        socratic_guidance: bottom_chunks[0],
        code_editor: bottom_chunks[1],
    }
}
