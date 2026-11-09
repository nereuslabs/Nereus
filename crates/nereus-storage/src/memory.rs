//! In-memory volatile storage backend for hermetic tests and fast ephemeral runs.

use async_trait::async_trait;
use nereus_core::error::CoreResult;
use nereus_core::state::StateMachineContext;
use nereus_core::traits::StorageBackend;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    sessions: Arc<RwLock<HashMap<Uuid, StateMachineContext>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn save_session(&self, ctx: &StateMachineContext) -> CoreResult<()> {
        let mut guard = self.sessions.write().await;
        guard.insert(ctx.session_id, ctx.clone());
        Ok(())
    }

    async fn load_session(&self, session_id: Uuid) -> CoreResult<Option<StateMachineContext>> {
        let guard = self.sessions.read().await;
        Ok(guard.get(&session_id).cloned())
    }
}
