//! Algebraic Finite State Machine (FSM) replacing LangGraph.

use crate::models::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningState {
    /// Diagnostic quiz probing user baseline before roadmap generation
    Diagnostic {
        questions: Vec<DiagnosticQuestion>,
        current_index: usize,
        answers: Vec<String>,
    },
    /// Generating prioritized roadmap based on diagnostic weakness report
    Coaching {
        weakness_report: WeaknessReport,
    },
    /// Presenting bite-sized theory and hands-on coding challenge
    Tutoring {
        topic: RoadmapTopic,
        material: String,
        task: String,
        retry_count: usize,
    },
    /// Running sandbox test suite and assessing user submission
    Examining {
        topic: RoadmapTopic,
        task: String,
        submission: Option<String>,
        assessment: Option<Assessment>,
    },
    /// All topics in roadmap mastered!
    Completed,
}

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
}
