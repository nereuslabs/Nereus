//! Deterministic offline Stub LLM client for hermetic CI and testbeds.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::traits::LLMClient;
use serde::de::DeserializeOwned;

#[derive(Debug, Default, Clone)]
pub struct StubLLMClient;

impl StubLLMClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LLMClient for StubLLMClient {
    async fn complete(&self, _prompt: &str) -> CoreResult<String> {
        Ok("Stub assessment: Task executed with PASS verdict. Mastery confirmed.".to_string())
    }

    async fn structured_output<T: DeserializeOwned + Send>(&self, _prompt: &str) -> CoreResult<T> {
        // Return a serialized mock or parse default JSON
        let empty_json = "{}";
        serde_json::from_str(empty_json)
            .map_err(|e| CoreError::Serialization(e.to_string()))
    }
}
