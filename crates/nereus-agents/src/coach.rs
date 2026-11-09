//! Coach Agent: Generates and orders the learning roadmap.

use async_trait::async_trait;
use nereus_core::models::{Roadmap, RoadmapTopic, UserProfile, WeaknessReport};
use nereus_core::traits::Agent;

#[derive(Debug, Default)]
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
                    title: "Memory Safety & RAII Semantics".to_string(),
                    description: "Understanding move semantics, ownership transfer, and drop traits.".to_string(),
                    difficulty: 0.3,
                    prerequisites: vec![],
                    estimated_hours: 2.0,
                },
                RoadmapTopic {
                    id: "topic-2".to_string(),
                    title: "Concurrency Primitives & Tokio Channels".to_string(),
                    description: "Multi-threaded state sharing via Arc<Mutex<T>> and async MPSC messaging.".to_string(),
                    difficulty: 0.7,
                    prerequisites: vec!["topic-1".to_string()],
                    estimated_hours: 4.0,
                },
            ],
        }
    }
}

#[async_trait]
impl Agent for CoachAgent {
    fn id(&self) -> &'static str {
        "coach-agent"
    }

    fn name(&self) -> &'static str {
        "Coach Agent"
    }
}
