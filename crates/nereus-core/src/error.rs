//! Standard error types for Nereus Core.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("State transition error: {0}")]
    InvalidTransition(String),

    #[error("Agent failure ({agent}): {message}")]
    AgentFailure {
        agent: String,
        message: String,
    },

    #[error("LLM provider error: {0}")]
    LLMError(String),

    #[error("Sandbox execution error: {0}")]
    SandboxError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Hot-swap failed: {0}")]
    HotSwapFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
