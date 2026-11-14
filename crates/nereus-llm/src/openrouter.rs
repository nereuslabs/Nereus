//! Live inference provider via OpenRouter API.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::traits::LLMClient;
use serde::de::DeserializeOwned;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct OpenRouterClient {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenRouterClient {
    pub fn new(api_key: impl Into<String>, model: Option<String>) -> Self {
        Self::with_base_url(
            "https://openrouter.ai/api/v1/chat/completions",
            api_key,
            model,
        )
    }

    pub fn with_base_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: Option<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.unwrap_or_else(|| "anthropic/claude-3.5-sonnet".to_string()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LLMClient for OpenRouterClient {
    async fn complete(&self, prompt: &str) -> CoreResult<String> {
        let payload = json!({
            "model": self.model,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let resp = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://nereuslabs.github.io")
            .header("X-Title", "Nereus 2.0")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CoreError::LLMError(format!(
                "OpenRouter error {status}: {text}"
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| CoreError::Serialization(e.to_string()))?;

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        Ok(content)
    }
}

impl OpenRouterClient {
    /// Request structured output deserialized from completion response.
    pub async fn structured_output<T: DeserializeOwned + Send>(
        &self,
        prompt: &str,
    ) -> CoreResult<T> {
        let raw = self.complete(prompt).await?;
        crate::client::parse_json::<T>(&raw).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}
