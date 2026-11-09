//! In-memory volatile storage backend for hermetic tests and fast ephemeral runs.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::state::StateMachineContext;
use nereus_core::traits::StorageBackend;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::models::{AssessmentRecord, SessionRecord, SessionStore};

#[derive(Debug, Default)]
struct MemoryStorageInner {
    sessions: HashMap<Uuid, SessionRecord>,
    assessments: HashMap<Uuid, Vec<AssessmentRecord>>,
}

/// Thread-safe in-memory storage backend tracking sessions and assessment logs.
#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    inner: Arc<RwLock<MemoryStorageInner>>,
}

impl MemoryStorage {
    /// Creates an empty in-memory storage instance.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MemoryStorageInner::default())),
        }
    }

    /// Atomically saves session state and an optional assessment record.
    pub async fn save_session_atomic(
        &self,
        ctx: &StateMachineContext,
        assessment: Option<&AssessmentRecord>,
    ) -> CoreResult<()> {
        let state_json =
            serde_json::to_string(ctx).map_err(|e| CoreError::Serialization(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();

        let record = SessionRecord::new(
            ctx.session_id,
            ctx.profile.skill.clone(),
            ctx.current_topic_index,
            state_json,
            now,
        );

        let mut guard = self.inner.write().await;
        guard.sessions.insert(ctx.session_id, record);

        if let Some(assess) = assessment {
            guard
                .assessments
                .entry(assess.session_id)
                .or_default()
                .push(assess.clone());
        }

        Ok(())
    }

    /// Convenience wrapper for atomically saving state and a new assessment record.
    pub async fn save_session_with_assessment(
        &self,
        ctx: &StateMachineContext,
        assessment: &AssessmentRecord,
    ) -> CoreResult<()> {
        self.save_session_atomic(ctx, Some(assessment)).await
    }

    /// Appends a new assessment record for a session.
    pub async fn record_assessment(&self, record: &AssessmentRecord) -> CoreResult<()> {
        let mut guard = self.inner.write().await;
        guard
            .assessments
            .entry(record.session_id)
            .or_default()
            .push(record.clone());
        Ok(())
    }

    /// Lists all tracked sessions ordered by most recently updated first.
    pub async fn list_sessions(&self) -> CoreResult<Vec<SessionRecord>> {
        let guard = self.inner.read().await;
        let mut sessions: Vec<SessionRecord> = guard.sessions.values().cloned().collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    /// Retrieves chronological assessment history for a specific session ID.
    pub async fn get_assessment_history(
        &self,
        session_id: Uuid,
    ) -> CoreResult<Vec<AssessmentRecord>> {
        let guard = self.inner.read().await;
        let history = guard
            .assessments
            .get(&session_id)
            .cloned()
            .unwrap_or_default();
        Ok(history)
    }

    /// Deletes a session and its associated assessment history.
    pub async fn delete_session(&self, session_id: Uuid) -> CoreResult<bool> {
        let mut guard = self.inner.write().await;
        let removed = guard.sessions.remove(&session_id).is_some();
        guard.assessments.remove(&session_id);
        Ok(removed)
    }

    /// Returns the number of currently tracked sessions.
    pub async fn len(&self) -> usize {
        let guard = self.inner.read().await;
        guard.sessions.len()
    }

    /// Returns true if no sessions are currently stored.
    pub async fn is_empty(&self) -> bool {
        let guard = self.inner.read().await;
        guard.sessions.is_empty()
    }

    /// Clears all stored sessions and assessment logs.
    pub async fn clear(&self) {
        let mut guard = self.inner.write().await;
        guard.sessions.clear();
        guard.assessments.clear();
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn save_session(&self, ctx: &StateMachineContext) -> CoreResult<()> {
        self.save_session_atomic(ctx, None).await
    }

    async fn load_session(&self, session_id: Uuid) -> CoreResult<Option<StateMachineContext>> {
        let guard = self.inner.read().await;
        if let Some(record) = guard.sessions.get(&session_id) {
            let ctx = record.context()?;
            Ok(Some(ctx))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl SessionStore for MemoryStorage {
    async fn list_sessions(&self) -> CoreResult<Vec<SessionRecord>> {
        self.list_sessions().await
    }

    async fn get_assessment_history(&self, session_id: Uuid) -> CoreResult<Vec<AssessmentRecord>> {
        self.get_assessment_history(session_id).await
    }

    async fn record_assessment(&self, record: &AssessmentRecord) -> CoreResult<()> {
        self.record_assessment(record).await
    }

    async fn save_session_atomic(
        &self,
        ctx: &StateMachineContext,
        assessment: Option<&AssessmentRecord>,
    ) -> CoreResult<()> {
        self.save_session_atomic(ctx, assessment).await
    }

    async fn delete_session(&self, session_id: Uuid) -> CoreResult<bool> {
        self.delete_session(session_id).await
    }
}
