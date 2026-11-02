//! Tutor Agent: Delivers conceptual materials and hands-on coding challenges.

use crate::models::RoadmapTopic;

pub struct TutorAgent;

impl TutorAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn provide_material(&self, topic: &RoadmapTopic) -> (String, String) {
        let material = format!(
            "# {}\n\n{}",
            topic.title, topic.description
        );
        let task = format!(
            "Demonstrate hands-on mastery of «{}» by implementing the required challenge.",
            topic.title
        );
        (material, task)
    }

    pub fn provide_revision(&self, topic: &RoadmapTopic, weak_areas: &[String]) -> (String, String) {
        let material = format!(
            "# Revision: {}\n\nFocusing on weak points: {}",
            topic.title,
            weak_areas.join(", ")
        );
        let task = format!("Solve the revised challenge for «{}».", topic.title);
        (material, task)
    }
}
