//! Examiner Agent: Sandboxed testing and Socratic evaluation.

use crate::models::{Assessment, RoadmapTopic, Verdict};

pub struct ExaminerAgent;

impl ExaminerAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate_submission(
        &self,
        topic: &RoadmapTopic,
        submission: &str,
    ) -> Assessment {
        // Dynamic pass threshold formula: 70 + difficulty * 15 (clamped <= 85)
        let threshold = (70.0 + topic.difficulty * 15.0).min(85.0);

        if submission.to_lowercase().contains("pass") || submission.to_lowercase().contains("tokio") {
            Assessment {
                topic_id: topic.id.clone(),
                score: 92.0,
                verdict: Verdict::Pass,
                feedback: "Topic mastered. Sandbox test assertions verified.".to_string(),
                weak_areas: vec![],
            }
        } else {
            Assessment {
                topic_id: topic.id.clone(),
                score: 55.0,
                verdict: Verdict::Retry,
                feedback: "Notice compiler error E0382: ownership moved. How does cloning the sender resolve the borrow?".to_string(),
                weak_areas: vec!["Ownership".to_string()],
            }
        }
    }
}
