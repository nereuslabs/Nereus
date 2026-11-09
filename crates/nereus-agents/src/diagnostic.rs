//! Diagnostic Agent: Evaluates baseline skills before roadmap generation.

use async_trait::async_trait;
use nereus_core::models::{DiagnosticQuestion, UserProfile, WeaknessReport};
use nereus_core::traits::Agent;

#[derive(Debug, Default)]
pub struct DiagnosticAgent;

impl DiagnosticAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_questions(&self, profile: &UserProfile) -> Vec<DiagnosticQuestion> {
        vec![
            DiagnosticQuestion {
                id: "diag-1".to_string(),
                question: format!("In {}, how does the compiler guarantee memory safety without a garbage collector?", profile.skill),
                options: vec![
                    "Through manual free() calls injected at compile time".to_string(),
                    "Through affine type system, ownership rules, and lifetime analysis".to_string(),
                    "By allocating all objects on a hidden global heap".to_string(),
                ],
                correct_index: Some(1),
            },
            DiagnosticQuestion {
                id: "diag-2".to_string(),
                question: "What is the key difference between std::sync::mpsc and tokio::sync::mpsc?".to_string(),
                options: vec![
                    "Tokio's channel send/recv operations are async non-blocking futures".to_string(),
                    "Std mpsc supports multiple consumers, while Tokio does not".to_string(),
                    "Tokio mpsc cannot handle bounded buffers".to_string(),
                ],
                correct_index: Some(0),
            },
        ]
    }

    pub fn evaluate_answers(&self, _answers: &[String]) -> WeaknessReport {
        WeaknessReport {
            weak_areas: vec!["Async Tokio".to_string(), "Lifetime Bounds".to_string()],
            recommended_topics: vec!["Pin & Future".to_string(), "Tokio Tasks & Channels".to_string()],
        }
    }
}

#[async_trait]
impl Agent for DiagnosticAgent {
    fn id(&self) -> &'static str {
        "diagnostic-agent"
    }

    fn name(&self) -> &'static str {
        "Diagnostic Agent"
    }
}
