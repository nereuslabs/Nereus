//! Core traits defining modular contracts and hot-swappable boundaries.

use crate::error::CoreResult;
use crate::models::{RoadmapTopic, SandboxExecutionResult};
use crate::state::StateMachineContext;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use uuid::Uuid;

/// Contract for asynchronous LLM completion.
#[async_trait]
pub trait LLMClient: Send + Sync + Debug {
    /// Request unstructured completion
    async fn complete(&self, prompt: &str) -> CoreResult<String>;
}

impl dyn LLMClient {
    /// Request structured output deserialized into type T from completion response.
    pub async fn structured_output<T: DeserializeOwned + Send>(
        &self,
        prompt: &str,
    ) -> CoreResult<T> {
        let raw = self.complete(prompt).await?;
        serde_json::from_str(&raw)
            .map_err(|e| crate::error::CoreError::Serialization(e.to_string()))
    }
}

/// Contract for isolated code compilation and execution.
#[async_trait]
pub trait SandboxRunner: Send + Sync + Debug {
    /// Compiles and runs student code against topic test suite
    async fn execute(&self, topic: &RoadmapTopic, code: &str)
        -> CoreResult<SandboxExecutionResult>;
}

/// Contract for session persistence across application runs.
#[async_trait]
pub trait StorageBackend: Send + Sync + Debug {
    /// Save active state machine context
    async fn save_session(&self, ctx: &StateMachineContext) -> CoreResult<()>;

    /// Load saved context by session UUID
    async fn load_session(&self, session_id: Uuid) -> CoreResult<Option<StateMachineContext>>;
}

/// Generic trait for specialized autonomous agents.
#[async_trait]
pub trait Agent: Send + Sync + Debug {
    /// Unique identifier for this agent implementation
    fn id(&self) -> &'static str;

    /// Human-readable display title
    fn name(&self) -> &'static str;
}

/// Contract for modules supporting runtime hot-swapping without engine restarts.
#[async_trait]
pub trait HotSwappable: Send + Sync + Debug {
    /// Export active state to a JSON string before unloading
    fn export_state(&self) -> CoreResult<String>;

    /// Import and restore state after hot-reloading
    fn import_state(&mut self, state_json: &str) -> CoreResult<()>;

    /// Version identifier of the module implementation
    fn module_version(&self) -> &'static str;
}
