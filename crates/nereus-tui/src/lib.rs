//! Nereus TUI: Terminal Cockpit interface using Ratatui.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod cockpit;
pub mod events;
pub mod layout;
pub mod widgets;

pub use cockpit::{CockpitApp, FocusedPane};
pub use events::{AppAction, AppEvent, EventHandler};
pub use layout::{calculate_cockpit_layout, CockpitPanes};
pub use widgets::{
    highlight_rust_line, EditorState, EditorWidget, LighthouseWidget, RoadmapState, RoadmapWidget,
    TheoryViewerWidget, DEFAULT_CODE_BOILERPLATE, TOTAL_LIGHTHOUSE_FRAMES,
};
