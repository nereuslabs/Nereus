//! Nereus TUI: Terminal Cockpit interface using Ratatui.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod cockpit;
pub mod layout;

pub use cockpit::CockpitApp;
pub use layout::{calculate_cockpit_layout, CockpitPanes};
