//! Tutor Agent: Generates conceptual reading materials and coding challenges.

use async_trait::async_trait;
use nereus_core::models::RoadmapTopic;
use nereus_core::traits::Agent;

#[derive(Debug, Default)]
pub struct TutorAgent;

impl TutorAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn provide_material(&self, topic: &RoadmapTopic) -> (String, String) {
        let material = format!("# {}\n\n{}\n\n### Key Concepts\n- Memory layout\n- Invariants\n- Ownership transfer", topic.title, topic.description);
        let task = format!("Demonstrate mastery of «{}» by writing a thread-safe implementation that passes all assertions.", topic.title);
        (material, task)
    }

    pub fn provide_revision(&self, topic: &RoadmapTopic, weak_areas: &[String]) -> (String, String) {
        let material = format!(
            "# Revision: {}\n\nFocusing on identified gaps: {}\n\nRecall how the borrow checker enforces aliasing XOR mutability.",
            topic.title,
            weak_areas.join(", ")
        );
        let task = format!("Refactor your solution for «{}» to resolve compiler errors and satisfy borrow rules.", topic.title);
        (material, task)
    }
}

#[async_trait]
impl Agent for TutorAgent {
    fn id(&self) -> &'static str {
        "tutor-agent"
    }

    fn name(&self) -> &'static str {
        "Tutor Agent"
    }
}
