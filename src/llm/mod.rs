//! Async LLM Client trait and deterministic offline stub.

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Provider unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, LLMError>;
}

/// Deterministic offline client for hermetic CI and local testing without API keys.
pub struct StubLLMClient;

#[async_trait]
impl LLMClient for StubLLMClient {
    async fn complete(&self, _prompt: &str) -> Result<String, LLMError> {
        Ok("Stub assessment: Task executed with PASS verdict. Mastery confirmed.".to_string())
    }
}
