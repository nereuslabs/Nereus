//! Thread-safe dynamic module registry enabling Tier-1 hot-switching.

use nereus_core::traits::{LLMClient, SandboxRunner, StorageBackend};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Container holding dynamically dispatched, hot-swappable subsystem components.
#[derive(Clone)]
pub struct ModuleRegistry {
    llm: Arc<RwLock<Arc<dyn LLMClient>>>,
    sandbox: Arc<RwLock<Arc<dyn SandboxRunner>>>,
    storage: Arc<RwLock<Arc<dyn StorageBackend>>>,
}

impl ModuleRegistry {
    pub fn new(
        llm: Arc<dyn LLMClient>,
        sandbox: Arc<dyn SandboxRunner>,
        storage: Arc<dyn StorageBackend>,
    ) -> Self {
        Self {
            llm: Arc::new(RwLock::new(llm)),
            sandbox: Arc::new(RwLock::new(sandbox)),
            storage: Arc::new(RwLock::new(storage)),
        }
    }

    /// Access the currently active LLM client
    pub async fn get_llm(&self) -> Arc<dyn LLMClient> {
        self.llm.read().await.clone()
    }

    /// Access the currently active sandbox runner
    pub async fn get_sandbox(&self) -> Arc<dyn SandboxRunner> {
        self.sandbox.read().await.clone()
    }

    /// Access the currently active storage backend
    pub async fn get_storage(&self) -> Arc<dyn StorageBackend> {
        self.storage.read().await.clone()
    }

    /// Hot-switch the active LLM provider at runtime without restarting the FSM loop
    pub async fn hot_switch_llm(&self, new_client: Arc<dyn LLMClient>) {
        let mut guard = self.llm.write().await;
        info!(
            "Hot-switching LLM provider: {:?} -> {:?}",
            *guard, new_client
        );
        *guard = new_client;
    }

    /// Hot-switch the active execution sandbox at runtime
    pub async fn hot_switch_sandbox(&self, new_sandbox: Arc<dyn SandboxRunner>) {
        let mut guard = self.sandbox.write().await;
        info!(
            "Hot-switching Sandbox runner: {:?} -> {:?}",
            *guard, new_sandbox
        );
        *guard = new_sandbox;
    }

    /// Hot-switch the active storage backend at runtime
    pub async fn hot_switch_storage(&self, new_storage: Arc<dyn StorageBackend>) {
        let mut guard = self.storage.write().await;
        info!(
            "Hot-switching Storage backend: {:?} -> {:?}",
            *guard, new_storage
        );
        *guard = new_storage;
    }
}
