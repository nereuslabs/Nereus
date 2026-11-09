//! Deterministic offline Stub LLM client for hermetic CI and testbeds.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::traits::LLMClient;
use serde::de::DeserializeOwned;

/// Deterministic Stub LLM client that requires zero API keys and zero tokens.
#[derive(Debug, Default, Clone)]
pub struct StubLLMClient {
    canned_completion: Option<String>,
    canned_json: Option<String>,
}

impl StubLLMClient {
    /// Creates a new default StubLLMClient with canned pass messages.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures a custom canned text completion response.
    pub fn with_canned_completion(mut self, text: impl Into<String>) -> Self {
        self.canned_completion = Some(text.into());
        self
    }

    /// Configures a custom canned JSON response for structured output calls.
    pub fn with_canned_json(mut self, json: impl Into<String>) -> Self {
        self.canned_json = Some(json.into());
        self
    }

    /// Request structured output deserialized from canned JSON response.
    pub async fn structured_output<T: DeserializeOwned + Send>(
        &self,
        _prompt: &str,
    ) -> CoreResult<T> {
        let json_str = self.canned_json.as_deref().unwrap_or("{}");
        crate::client::parse_json::<T>(json_str)
            .map_err(|e| CoreError::Serialization(e.to_string()))
    }
}

#[async_trait]
impl LLMClient for StubLLMClient {
    async fn complete(&self, _prompt: &str) -> CoreResult<String> {
        if let Some(ref text) = self.canned_completion {
            Ok(text.clone())
        } else {
            Ok("Stub assessment: Task executed with PASS verdict. Mastery confirmed.".to_string())
        }
    }
}
