//! Domain models and traits for session discovery and assessment history.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::models::{Assessment, Verdict};
use nereus_core::state::StateMachineContext;
use nereus_core::traits::StorageBackend;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Lightweight summary and persisted record of a state machine session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: Uuid,
    pub user_skill: String,
    pub current_topic_index: usize,
    pub state_json: String,
    pub updated_at: String,
}

impl SessionRecord {
    /// Constructs a new `SessionRecord`.
    pub fn new(
        session_id: Uuid,
        user_skill: impl Into<String>,
        current_topic_index: usize,
        state_json: impl Into<String>,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            session_id,
            user_skill: user_skill.into(),
            current_topic_index,
            state_json: state_json.into(),
            updated_at: updated_at.into(),
        }
    }

    /// Deserializes the underlying JSON state into a full `StateMachineContext`.
    pub fn context(&self) -> CoreResult<StateMachineContext> {
        serde_json::from_str(&self.state_json).map_err(|e| {
            CoreError::Serialization(format!("Failed to parse session state JSON: {e}"))
        })
    }
}

/// Historical assessment record representing a student's evaluation on a topic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssessmentRecord {
    pub id: String,
    pub session_id: Uuid,
    pub topic_id: String,
    pub score: f32,
    pub verdict: String,
    pub attempts: usize,
    pub feedback: String,
    pub timestamp: String,
}

impl AssessmentRecord {
    /// Creates a new `AssessmentRecord` with an auto-generated UUID and UTC timestamp.
    pub fn new(
        session_id: Uuid,
        topic_id: impl Into<String>,
        score: f32,
        verdict: impl Into<String>,
        attempts: usize,
        feedback: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            topic_id: topic_id.into(),
            score,
            verdict: verdict.into(),
            attempts,
            feedback: feedback.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Creates an `AssessmentRecord` with explicit ID and timestamp.
    #[allow(clippy::too_many_arguments)]
    pub fn with_id(
        id: impl Into<String>,
        session_id: Uuid,
        topic_id: impl Into<String>,
        score: f32,
        verdict: impl Into<String>,
        attempts: usize,
        feedback: impl Into<String>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            session_id,
            topic_id: topic_id.into(),
            score,
            verdict: verdict.into(),
            attempts,
            feedback: feedback.into(),
            timestamp: timestamp.into(),
        }
    }

    /// Constructs an `AssessmentRecord` from a core `Assessment` domain model.
    pub fn from_assessment(session_id: Uuid, assessment: &Assessment, attempts: usize) -> Self {
        let verdict_str = match assessment.verdict {
            Verdict::Pass => "pass",
            Verdict::Retry => "retry",
        };

        Self::new(
            session_id,
            assessment.topic_id.clone(),
            assessment.score,
            verdict_str,
            attempts,
            assessment.feedback.clone(),
        )
    }

    /// Parses the verdict string into a core `Verdict` enum if valid.
    pub fn verdict_enum(&self) -> Option<Verdict> {
        match self.verdict.to_lowercase().as_str() {
            "pass" => Some(Verdict::Pass),
            "retry" => Some(Verdict::Retry),
            _ => None,
        }
    }
}

/// Extended storage contract supporting session discovery, history queries, and atomic checkpoints.
#[async_trait]
pub trait SessionStore: StorageBackend {
    /// List all persisted sessions, ordered by last update descending.
    async fn list_sessions(&self) -> CoreResult<Vec<SessionRecord>>;

    /// Retrieve the chronological assessment history for a specific session.
    async fn get_assessment_history(&self, session_id: Uuid) -> CoreResult<Vec<AssessmentRecord>>;

    /// Atomically persist a topic assessment entry to history.
    async fn record_assessment(&self, record: &AssessmentRecord) -> CoreResult<()>;

    /// Atomically persist or update a session state, optionally recording an assessment in the same transaction.
    async fn save_session_atomic(
        &self,
        ctx: &StateMachineContext,
        assessment: Option<&AssessmentRecord>,
    ) -> CoreResult<()>;

    /// Deletes a session and its associated assessment history.
    async fn delete_session(&self, session_id: Uuid) -> CoreResult<bool>;
}
