//! Domain models for Nereus 2.0 learning automaton.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserLevel {
    Beginner,
    Intermediate,
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub skill: String,
    pub current_level: UserLevel,
    pub target_level: UserLevel,
    pub hours_per_day: f32,
    pub deadline_days: u32,
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapTopic {
    pub id: String,
    pub title: String,
    pub description: String,
    pub difficulty: f32,
    pub prerequisites: Vec<String>,
    pub estimated_hours: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Roadmap {
    pub topics: Vec<RoadmapTopic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticQuestion {
    pub id: String,
    pub question: String,
    pub options: Vec<String>,
    pub correct_index: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeaknessReport {
    pub weak_areas: Vec<String>,
    pub recommended_topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    pub topic_id: String,
    pub score: f32,
    pub verdict: Verdict,
    pub feedback: String,
    pub weak_areas: Vec<String>,
}
