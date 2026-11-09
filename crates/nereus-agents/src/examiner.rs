//! Examiner Agent: Assesses sandbox execution output and delivers Socratic guidance.

use async_trait::async_trait;
use nereus_core::models::{Assessment, RoadmapTopic, SandboxExecutionResult, Verdict};
use nereus_core::traits::Agent;

#[derive(Debug, Default)]
pub struct ExaminerAgent;

impl ExaminerAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn assess_execution(
        &self,
        topic: &RoadmapTopic,
        sandbox: &SandboxExecutionResult,
    ) -> Assessment {
        let threshold = (70.0 + topic.difficulty * 15.0).min(85.0);

        if sandbox.success {
            Assessment {
                topic_id: topic.id.clone(),
                score: 95.0,
                verdict: Verdict::Pass,
                feedback: "Topic mastered. Sandbox test assertions and compiler checks verified.".to_string(),
                weak_areas: vec![],
            }
        } else {
            let feedback = if sandbox.stderr.contains("E0382") {
                "Notice compiler error E0382: value borrowed here after move. How does Arc::clone or borrowing borrow the instance without consuming it?".to_string()
            } else if sandbox.stderr.contains("assertion failed") {
                "A test assertion failed. Check the boundary condition when the buffer is empty.".to_string()
            } else {
                "The solution did not pass execution verification. Check type signatures and compiler diagnostics.".to_string()
            };

            Assessment {
                topic_id: topic.id.clone(),
                score: 55.0,
                verdict: Verdict::Retry,
                feedback,
                weak_areas: vec!["Borrow Checker".to_string()],
            }
        }
    }
}

#[async_trait]
impl Agent for ExaminerAgent {
    fn id(&self) -> &'static str {
        "examiner-agent"
    }

    fn name(&self) -> &'static str {
        "Examiner Agent"
    }
}
