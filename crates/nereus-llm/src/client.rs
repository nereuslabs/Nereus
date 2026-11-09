//! Structured inference client with automatic retry logic on JSON parsing failures.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::traits::LLMClient;
use serde::de::DeserializeOwned;
use tracing::{debug, warn};

/// Extracts JSON payload from potentially markdown-fenced or decorated text.
pub fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();

    // Check for markdown code fences with ```json ... ```
    if let Some(start) = trimmed.find("```json") {
        let after_start = &trimmed[start + 7..];
        if let Some(end) = after_start.rfind("```") {
            return after_start[..end].trim();
        }
    }

    // Check for generic markdown code fences ``` ... ```
    if let Some(start) = trimmed.find("```") {
        let after_start = &trimmed[start + 3..];
        if let Some(end) = after_start.rfind("```") {
            return after_start[..end].trim();
        }
    }

    // Direct object or array slice if braces are present
    if let (Some(first_brace), Some(last_brace)) = (trimmed.find('{'), trimmed.rfind('}')) {
        let obj_slice = &trimmed[first_brace..=last_brace];
        if let (Some(first_bracket), Some(last_bracket)) = (trimmed.find('['), trimmed.rfind(']')) {
            if first_bracket < first_brace && last_bracket > last_brace {
                return &trimmed[first_bracket..=last_bracket];
            }
        }
        return obj_slice;
    } else if let (Some(first_bracket), Some(last_bracket)) =
        (trimmed.find('['), trimmed.rfind(']'))
    {
        return &trimmed[first_bracket..=last_bracket];
    }

    trimmed
}

/// Helper function to parse JSON with fallback to [`extract_json`].
pub fn parse_json<T: DeserializeOwned>(raw: &str) -> Result<T, serde_json::Error> {
    let trimmed = raw.trim();
    match serde_json::from_str::<T>(trimmed) {
        Ok(parsed) => Ok(parsed),
        Err(direct_err) => {
            let extracted = extract_json(trimmed);
            if extracted != trimmed {
                serde_json::from_str::<T>(extracted).map_err(|_| direct_err)
            } else {
                Err(direct_err)
            }
        }
    }
}

/// Structured inference client that wraps an underlying [`LLMClient`].
///
/// If JSON parsing fails, it performs up to `max_retries` (default: 2) retry attempts
/// with contextual error feedback ("Previous output was invalid JSON: {error}").
#[derive(Debug, Clone)]
pub struct StructuredClient<C: LLMClient> {
    inner: C,
    max_retries: usize,
}

impl<C: LLMClient> StructuredClient<C> {
    /// Creates a new `StructuredClient` wrapping the given client with default 2 retries.
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            max_retries: 2,
        }
    }

    /// Configures the maximum number of retry attempts.
    pub fn with_max_retries(inner: C, max_retries: usize) -> Self {
        Self { inner, max_retries }
    }

    /// Access reference to the inner client.
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// Access mutable reference to the inner client.
    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    /// Consume wrapper and return the inner client.
    pub fn into_inner(self) -> C {
        self.inner
    }

    /// Executes structured output parsing with up to `max_retries` retries.
    pub async fn structured_output_with_retry<T: DeserializeOwned + Send>(
        &self,
        prompt: &str,
    ) -> CoreResult<T> {
        let mut current_prompt = prompt.to_string();
        let mut last_error_message = String::new();

        // 1 initial attempt + up to max_retries retry attempts
        let total_attempts = self.max_retries + 1;

        for attempt in 1..=total_attempts {
            debug!("Structured inference attempt {attempt}/{total_attempts}");
            let raw_response = self.inner.complete(&current_prompt).await?;

            match parse_json::<T>(&raw_response) {
                Ok(parsed) => {
                    if attempt > 1 {
                        debug!("Successfully parsed structured output after {attempt} attempts");
                    }
                    return Ok(parsed);
                }
                Err(err) => {
                    let err_str = err.to_string();
                    warn!("Attempt {attempt}/{total_attempts} failed to parse JSON: {err_str}");
                    last_error_message = err_str.clone();

                    if attempt < total_attempts {
                        // Construct the retry prompt with required feedback string
                        current_prompt = format!(
                            "{prompt}\n\nPrevious output was invalid JSON: {err_str}\nPlease format your response as strictly valid JSON matching the requested schema."
                        );
                    }
                }
            }
        }

        Err(CoreError::Serialization(format!(
            "Previous output was invalid JSON: {last_error_message}"
        )))
    }
}

#[async_trait]
impl<C: LLMClient> LLMClient for StructuredClient<C> {
    async fn complete(&self, prompt: &str) -> CoreResult<String> {
        self.inner.complete(prompt).await
    }

    async fn structured_output<T: DeserializeOwned + Send>(&self, prompt: &str) -> CoreResult<T> {
        self.structured_output_with_retry(prompt).await
    }
}
