//! Ratatui widgets for the Nereus TUI cockpit.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod editor;
pub mod help;
pub mod lighthouse;
pub mod provider;
pub mod quiz;
pub mod roadmap;
pub mod status_bar;
pub mod theory;

pub use editor::{highlight_rust_line, EditorState, EditorWidget, DEFAULT_CODE_BOILERPLATE};
pub use help::{centered_rect, HelpWidget};
pub use lighthouse::{LighthouseWidget, TOTAL_LIGHTHOUSE_FRAMES};
pub use provider::{ProviderDialogWidget, ProviderOption, ProviderState, AVAILABLE_PROVIDERS};
pub use quiz::{DiagnosticQuizWidget, QuizState};
pub use roadmap::{RoadmapState, RoadmapWidget};
pub use status_bar::StatusBarWidget;
pub use theory::TheoryViewerWidget;
