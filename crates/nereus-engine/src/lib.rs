//! Nereus Engine: Finite state machine orchestrator, transition runner, and evaluator.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod decay;
pub mod evaluator;
pub mod fsm;

pub use decay::{EbbinghausDecayEngine, DEFAULT_DECAY_THRESHOLD, MINIMUM_STABILITY_DAYS};
pub use evaluator::{compute_pass_threshold, evaluate_verdict, sanitize_socratic_feedback};
pub use fsm::LearningAutomaton;
