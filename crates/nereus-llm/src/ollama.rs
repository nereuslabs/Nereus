//! Local inference provider via Ollama daemon.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::traits::LLMClient;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434";
pub const DEFAULT_OLLAMA_MODEL: &str = "qwen2.5-coder:1.5b";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaGenerateResponse {
    pub response: String,
    pub done: bool,
}

/// Inference client connecting to a locally running Ollama daemon.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    pub endpoint: String,
    pub model: String,
    pub client: reqwest::Client,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new(DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL)
    }
}

impl OllamaClient {
    pub fn new(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(model: impl Into<String>) -> Self {
        Self::new(DEFAULT_OLLAMA_ENDPOINT, model)
    }

    /// Checks whether the local Ollama daemon is reachable.
    pub async fn is_available(&self) -> bool {
        let health_url = format!("{}/api/tags", self.endpoint.trim_end_matches('/'));
        self.client.get(&health_url).send().await.is_ok()
    }
}

#[async_trait]
impl LLMClient for OllamaClient {
    async fn complete(&self, prompt: &str) -> CoreResult<String> {
        let url = format!("{}/api/generate", self.endpoint.trim_end_matches('/'));
        let payload = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                CoreError::LLMError(format!(
                    "Failed to connect to local Ollama daemon at {}: {}",
                    self.endpoint, e
                ))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CoreError::LLMError(format!(
                "Ollama daemon returned HTTP {}: {}",
                status, body
            )));
        }

        let body: OllamaGenerateResponse = resp
            .json()
            .await
            .map_err(|e| CoreError::LLMError(format!("Invalid JSON from Ollama daemon: {}", e)))?;

        Ok(body.response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_client_initialization() {
        let client = OllamaClient::default();
        assert_eq!(client.endpoint, DEFAULT_OLLAMA_ENDPOINT);
        assert_eq!(client.model, DEFAULT_OLLAMA_MODEL);

        let custom = OllamaClient::new("http://127.0.0.1:11434", "llama3.2:1b");
        assert_eq!(custom.endpoint, "http://127.0.0.1:11434");
        assert_eq!(custom.model, "llama3.2:1b");
    }

    #[test]
    fn test_ollama_response_deserialization() {
        let raw_json = r#"{"response":"Socratic reflection: consider ownership","done":true}"#;
        let parsed: Result<OllamaGenerateResponse, _> = serde_json::from_str(raw_json);
        assert!(parsed.is_ok());
        let res = parsed.unwrap();
        assert_eq!(res.response, "Socratic reflection: consider ownership");
        assert!(res.done);
    }
}
