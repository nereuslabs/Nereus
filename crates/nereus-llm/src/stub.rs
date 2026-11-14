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
    async fn complete(&self, prompt: &str) -> CoreResult<String> {
        if let Some(ref text) = self.canned_completion {
            Ok(text.clone())
        } else if prompt.contains("Compiler / Sandbox Diagnostics:") {
            // Socratic response recognizing error codes without leaking code
            if prompt.contains("E0382") {
                Ok("Stub assessment: Diagnostic: Variable used after move (E0382). Reflective Question: What happens to ownership when you pass a non-Copy type by value?".to_string())
            } else if prompt.contains("E0597") {
                Ok("Stub assessment: Diagnostic: Borrowed value does not live long enough (E0597). Reflective Question: How can you extend the lifetime of the referenced value?".to_string())
            } else if prompt.contains("E0277") {
                Ok("Stub assessment: Diagnostic: Trait bound not satisfied (E0277). Reflective Question: Which trait must be implemented or derived for this type?".to_string())
            } else if prompt.contains("E0502") {
                Ok("Stub assessment: Diagnostic: Cannot borrow as mutable because it is also borrowed as immutable (E0502). Reflective Question: When does the immutable borrow end?".to_string())
            } else if prompt.contains("E0106") {
                Ok("Stub assessment: Diagnostic: Missing lifetime specifier (E0106). Reflective Question: How are input and output lifetimes related in this function signature?".to_string())
            } else if prompt.contains("E0308") {
                Ok("Stub assessment: Diagnostic: Mismatched types (E0308). Reflective Question: What concrete type is being returned versus what the signature declares?".to_string())
            } else if prompt.contains("E0507") {
                Ok("Stub assessment: Diagnostic: Cannot move out of shared reference (E0507). Reflective Question: How can you access the inner value without transferring ownership?".to_string())
            } else if prompt.contains("E0499") {
                Ok("Stub assessment: Diagnostic: Multiple mutable borrows (E0499). Reflective Question: How can scopes be separated to prevent overlapping mutable borrows?".to_string())
            } else if prompt.contains("E0716") {
                Ok("Stub assessment: Diagnostic: Temporary value dropped while borrowed (E0716). Reflective Question: How can you bind the temporary value to a local variable to extend its lifetime?".to_string())
            } else if prompt.contains("E0282") {
                Ok("Stub assessment: Diagnostic: Type annotations needed (E0282). Reflective Question: Where can an explicit type annotation guide the compiler's type inference?".to_string())
            } else {
                Ok("Stub assessment: Diagnostic: Compiler error detected. Reflective Question: How can borrow checker constraints be satisfied here without violating memory safety?".to_string())
            }
        } else {
            Ok("Stub assessment: Task executed with PASS verdict. Mastery confirmed.".to_string())
        }
    }
}
