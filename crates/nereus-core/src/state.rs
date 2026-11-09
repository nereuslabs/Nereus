//! The Algebraic Finite State Machine (FSM) representation.

use crate::models::{Assessment, DiagnosticQuestion, Roadmap, RoadmapTopic, UserProfile, WeaknessReport};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Exhaustive compile-time state variants of the Nereus learning lifecycle.
/// Invalid transitions and impossible states are strictly unrepresentable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningState {
    /// Initial baseline probing phase before roadmap compilation
    Diagnostic {
        questions: Vec<DiagnosticQuestion>,
        current_index: usize,
        answers: Vec<String>,
    },
    /// Generating prioritized roadmap based on diagnostic weakness report
    Coaching {
        weakness_report: WeaknessReport,
    },
    /// Delivering concise theory and interactive challenge
    Tutoring {
        topic: RoadmapTopic,
        material: String,
        task: String,
        retry_count: usize,
    },
    /// Executing student code inside the sandbox and evaluating results
    Examining {
        topic: RoadmapTopic,
        task: String,
        submission: Option<String>,
        assessment: Option<Assessment>,
    },
    /// All milestones in the roadmap achieved
    Completed,
}

impl LearningState {
    pub fn name(&self) -> &'static str {
        match self {
            LearningState::Diagnostic { .. } => "Diagnostic",
            LearningState::Coaching { .. } => "Coaching",
            LearningState::Tutoring { .. } => "Tutoring",
            LearningState::Examining { .. } => "Examining",
            LearningState::Completed => "Completed",
        }
    }
}

/// The state machine context carrying active session metadata and current topic pointer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineContext {
    pub session_id: Uuid,
    pub profile: UserProfile,
    pub roadmap: Roadmap,
    pub current_topic_index: usize,
    pub state: LearningState,
}

impl StateMachineContext {
    pub fn new(profile: UserProfile, run_diagnostic: bool) -> Self {
        let state = if run_diagnostic {
            LearningState::Diagnostic {
                questions: vec![],
                current_index: 0,
                answers: vec![],
            }
        } else {
            LearningState::Coaching {
                weakness_report: WeaknessReport::default(),
            }
        };

        Self {
            session_id: Uuid::new_v4(),
            profile,
            roadmap: Roadmap::default(),
            current_topic_index: 0,
            state,
        }
    }

    pub fn current_topic(&self) -> Option<&RoadmapTopic> {
        self.roadmap.topics.get(self.current_topic_index)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.state, LearningState::Completed)
    }
}
