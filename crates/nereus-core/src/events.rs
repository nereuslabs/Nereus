//! Domain events emitted during FSM transitions.

use crate::models::{Assessment, RoadmapTopic, Verdict};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    SessionStarted {
        session_id: Uuid,
        skill: String,
    },
    DiagnosticQuestionPresented {
        index: usize,
        total: usize,
        question_id: String,
    },
    DiagnosticCompleted {
        weak_areas: Vec<String>,
    },
    RoadmapGenerated {
        total_topics: usize,
    },
    TopicStarted {
        topic_id: String,
        title: String,
    },
    CodeSubmitted {
        topic_id: String,
        code_length: usize,
    },
    AssessmentFinished {
        topic_id: String,
        score: f32,
        verdict: Verdict,
    },
    TopicAdvanced {
        new_index: usize,
    },
    TopicRetried {
        topic_id: String,
        retry_count: usize,
    },
    SessionCompleted {
        session_id: Uuid,
    },
}
