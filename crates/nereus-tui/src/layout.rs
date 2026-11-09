//! Terminal Cockpit layout manager.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone)]
pub struct CockpitPanes {
    pub roadmap: Rect,
    pub task: Rect,
    pub socratic_guidance: Rect,
    pub code_editor: Rect,
}

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
