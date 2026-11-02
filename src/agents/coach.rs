//! Coach Agent: Generates dynamic learning roadmap prioritizing weak areas.

use crate::models::{Roadmap, RoadmapTopic, UserProfile, WeaknessReport};

pub struct CoachAgent;

impl CoachAgent {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_roadmap(&self, _profile: &UserProfile, _weakness: &WeaknessReport) -> Roadmap {
        Roadmap {
            topics: vec![
                RoadmapTopic {
                    id: "topic-1".to_string(),
                    title: "Memory Safety & Ownership".to_string(),
                    description: "Deep dive into move semantics, borrowing, and RAII.".to_string(),
                    difficulty: 0.3,
                    prerequisites: vec![],
                    estimated_hours: 2.0,
                },
                RoadmapTopic {
                    id: "topic-2".to_string(),
                    title: "Concurrency Primitives & Tokio".to_string(),
                    description: "Mastering threads, Arc, Mutex, and async channels.".to_string(),
                    difficulty: 0.7,
                    prerequisites: vec!["topic-1".to_string()],
                    estimated_hours: 4.0,
                },
            ],
        }
    }
}
