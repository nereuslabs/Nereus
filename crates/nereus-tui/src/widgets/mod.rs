//! Ratatui widgets for the Nereus TUI cockpit.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod editor;
pub mod lighthouse;
pub mod roadmap;
pub mod theory;

pub use editor::{highlight_rust_line, EditorState, EditorWidget, DEFAULT_CODE_BOILERPLATE};
pub use lighthouse::{LighthouseWidget, TOTAL_LIGHTHOUSE_FRAMES};
pub use roadmap::{RoadmapState, RoadmapWidget};
pub use theory::TheoryViewerWidget;
