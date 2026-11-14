//! Nereus Core: Fundamental types, state machine representations, and traits.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod error;
pub mod events;
pub mod models;
pub mod state;
pub mod traits;

pub use error::{CoreError, CoreResult};
pub use events::DomainEvent;
pub use models::*;
pub use state::{LearningState, StateMachineContext};
pub use traits::{Agent, HotSwappable, LLMClient, SandboxRunner, StorageBackend};
