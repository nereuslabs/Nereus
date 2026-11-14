//! Direct inference provider via the Anthropic Messages API.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::traits::LLMClient;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::fmt::Display;

/// Supported Anthropic models for inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnthropicModel {
    /// Claude 3.5 Sonnet (default, balanced speed and reasoning)
    Claude35Sonnet,
    /// Claude 3 Opus (complex reasoning and deep analysis)
    Claude3Opus,
    /// Claude 3 Haiku (low-latency, compact micro-evaluations)
    Claude3Haiku,
    /// Custom or future model identifier
    Custom(String),
}

impl AnthropicModel {
    /// Returns the canonical API model identifier string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Claude35Sonnet => "claude-3-5-sonnet-20241022",
            Self::Claude3Opus => "claude-3-opus-20240229",
            Self::Claude3Haiku => "claude-3-haiku-20240307",
            Self::Custom(model) => model.as_str(),
        }
    }
}

impl Display for AnthropicModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for AnthropicModel {
    fn from(s: &str) -> Self {
        match s {
            "claude-3-5-sonnet" | "claude-3-5-sonnet-20241022" | "claude-3-5-sonnet-latest" => {
                Self::Claude35Sonnet
            }
            "claude-3-opus" | "claude-3-opus-20240229" | "claude-3-opus-latest" => {
                Self::Claude3Opus
            }
            "claude-3-haiku" | "claude-3-haiku-20240307" => Self::Claude3Haiku,
            custom => Self::Custom(custom.to_string()),
        }
    }
}

impl From<String> for AnthropicModel {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Client for communicating directly with the Anthropic Messages API.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    api_key: String,
    model: AnthropicModel,
    max_tokens: u32,
    client: reqwest::Client,
    base_url: String,
    system_prompt: Option<String>,
}

impl AnthropicClient {
    /// Default Anthropic Messages API endpoint.
    pub const DEFAULT_BASE_URL: &'static str = "https://api.anthropic.com/v1/messages";

    /// Standard Anthropic API version header.
    pub const ANTHROPIC_VERSION: &'static str = "2023-06-01";

    /// Default token limit for message generation.
    pub const DEFAULT_MAX_TOKENS: u32 = 4096;

    /// Creates a new `AnthropicClient` defaulting to Claude 3.5 Sonnet.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, AnthropicModel::Claude35Sonnet)
    }

    /// Creates a new `AnthropicClient` configured with a specific [`AnthropicModel`].
    pub fn with_model(api_key: impl Into<String>, model: impl Into<AnthropicModel>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            max_tokens: Self::DEFAULT_MAX_TOKENS,
            client: reqwest::Client::new(),
            base_url: Self::DEFAULT_BASE_URL.to_string(),
            system_prompt: None,
        }
    }

    /// Convenience constructor for Claude 3.5 Sonnet.
    pub fn claude_3_5_sonnet(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, AnthropicModel::Claude35Sonnet)
    }

    /// Convenience constructor for Claude 3 Opus.
    pub fn claude_3_opus(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, AnthropicModel::Claude3Opus)
    }

    /// Sets or changes the model for this client.
    pub fn set_model(mut self, model: impl Into<AnthropicModel>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets the maximum output token count.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Overrides the base URL (useful for testing or proxy environments).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sets an optional system prompt applied across calls.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system_prompt = Some(system.into());
        self
    }

    /// Returns the active model identifier string.
    pub fn model(&self) -> &str {
        self.model.as_str()
    }

    /// Returns the target API endpoint URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Sends a prompt to the Anthropic Messages API and returns the aggregated text response.
    async fn send_message(&self, prompt: &str) -> CoreResult<String> {
        let mut payload = json!({
            "model": self.model.as_str(),
            "max_tokens": self.max_tokens,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        if let Some(ref system) = self.system_prompt {
            payload["system"] = json!(system);
        }

        let resp = self
            .client
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", Self::ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CoreError::LLMError(format!("Anthropic network error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();

            // Attempt to parse structured Anthropic error response
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(err_obj) = val.get("error") {
                    let err_type = err_obj
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown_error");
                    let err_msg = err_obj
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or(&text);

                    return Err(CoreError::LLMError(format!(
                        "Anthropic API error ({status}): [{err_type}] {err_msg}"
                    )));
                }
            }

            return Err(CoreError::LLMError(format!(
                "Anthropic API error ({status}): {text}"
            )));
        }

        let body: serde_json::Value = resp.json().await.map_err(|e| {
            CoreError::Serialization(format!("Failed to parse Anthropic response JSON: {e}"))
        })?;

        let content_blocks = body["content"].as_array().ok_or_else(|| {
            CoreError::LLMError("Anthropic response missing 'content' array".to_string())
        })?;

        let mut text_parts = Vec::new();
        for block in content_blocks {
            if block["type"].as_str() == Some("text") {
                if let Some(text) = block["text"].as_str() {
                    text_parts.push(text);
                }
            }
        }

        if text_parts.is_empty() {
            return Err(CoreError::LLMError(
                "Anthropic response contained no text content blocks".to_string(),
            ));
        }

        Ok(text_parts.join("\n"))
    }

    /// Request structured output deserialized from Anthropic completion.
    pub async fn structured_output<T: DeserializeOwned + Send>(
        &self,
        prompt: &str,
    ) -> CoreResult<T> {
        let raw = self.complete(prompt).await?;
        crate::client::parse_json::<T>(&raw).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}

#[async_trait]
impl LLMClient for AnthropicClient {
    async fn complete(&self, prompt: &str) -> CoreResult<String> {
        self.send_message(prompt).await
    }
}
