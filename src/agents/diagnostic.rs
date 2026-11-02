//! Diagnostic Agent: Probes skill baseline before roadmap generation.

use crate::models::{DiagnosticQuestion, UserProfile, WeaknessReport};

pub struct DiagnosticAgent;

impl DiagnosticAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_questions(&self, profile: &UserProfile) -> Vec<DiagnosticQuestion> {
        vec![
            DiagnosticQuestion {
                id: "diag-1".to_string>,
                question: format!("In {}, how do you handle memory allocation and lifetime bounds?", profile.skill),
                options: vec![
                    "Using automatic garbage collection".to_string(),
                    "Explicit borrow checker and lifetime annotations".to_string(),
                    "Manual malloc and free exclusively".to_string(),
                ],
                correct_index: Some(1),
            },
        ]
    }

    pub fn evaluate_answers(&self, _answers: &[String]) -> WeaknessReport {
        WeaknessReport {
            weak_areas: vec!["Lifetimes".to_string(), "Concurrency".to_string()],
            recommended_topics: vec!["Async Tokio".to_string(), "MPSC Channels".to_string()],
        }
    }
}
