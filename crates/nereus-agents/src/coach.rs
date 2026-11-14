//! Coach Agent: Generates, prioritizes, and schedules the learning roadmap.
//!
//! Employs Directed Acyclic Graph (DAG) topological sorting with weakness-prioritization
//! heuristics and computes milestone schedules against study pace and deadlines.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use nereus_core::models::{Roadmap, RoadmapTopic, UserLevel, UserProfile, WeaknessReport};
use nereus_core::traits::Agent;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors arising during roadmap graph sorting or schedule calculation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoachError {
    #[error("Cyclic dependency detected in roadmap prerequisites among topics: {0:?}")]
    CycleDetected(Vec<String>),
    #[error("Topic '{topic_id}' references missing prerequisite '{prereq_id}'")]
    MissingPrerequisite { topic_id: String, prereq_id: String },
    #[error("Invalid schedule parameters: {0}")]
    InvalidSchedule(String),
}

/// Projected schedule milestone for a single roadmap topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneSchedule {
    pub topic_id: String,
    pub topic_title: String,
    pub estimated_hours: f32,
    pub cumulative_hours: f32,
    pub target_day: f32,
    pub target_date: DateTime<Utc>,
    pub within_deadline: bool,
}

/// Comprehensive roadmap schedule projection with feasibility analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapSchedule {
    pub milestones: Vec<MilestoneSchedule>,
    pub total_hours: f32,
    pub total_days: f32,
    pub deadline_days: u32,
    pub hours_per_day: f32,
    pub required_hours_per_day: f32,
    pub feasible: bool,
    pub pace_warning: Option<String>,
}

/// Coach Agent responsible for curriculum structuring and pace orchestration.
#[derive(Debug, Clone)]
pub struct CoachAgent {
    default_catalog: Vec<RoadmapTopic>,
}

impl Default for CoachAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl CoachAgent {
    /// Initialize CoachAgent with default curriculum topics from `nereus_sandbox::curriculum::Curriculum`.
    /// Falls back to the built-in topic catalog if the curriculum returns an empty list.
    pub fn new() -> Self {
        let curriculum_topics = nereus_sandbox::curriculum::Curriculum::default_topics();
        let default_catalog = if !curriculum_topics.is_empty() {
            curriculum_topics
        } else {
            Self::builtin_fallback_catalog()
        };

        Self { default_catalog }
    }

    /// Creates a CoachAgent with a custom topic catalog.
    pub fn with_catalog(default_catalog: Vec<RoadmapTopic>) -> Self {
        Self { default_catalog }
    }

    /// Built-in fallback catalog of foundational Rust topics.
    pub fn builtin_fallback_catalog() -> Vec<RoadmapTopic> {
        vec![
            RoadmapTopic {
                id: "topic-ownership".to_string(),
                title: "Memory Safety & RAII Semantics".to_string(),
                description: "Deep dive into move semantics, ownership transfer, affine types, and Drop.".to_string(),
                difficulty: 0.3,
                prerequisites: vec![],
                estimated_hours: 2.5,
            },
            RoadmapTopic {
                id: "topic-borrowing".to_string(),
                title: "Borrow Checker & Exclusive Mutability".to_string(),
                description: "Aliasing XOR mutability, reference mechanics, reborrowing, and non-lexical lifetimes.".to_string(),
                difficulty: 0.4,
                prerequisites: vec!["topic-ownership".to_string()],
                estimated_hours: 3.0,
            },
            RoadmapTopic {
                id: "topic-error".to_string(),
                title: "Robust Error Handling with thiserror".to_string(),
                description: "Error propagation via ?, typed custom error hierarchies, Result/Option combinators.".to_string(),
                difficulty: 0.35,
                prerequisites: vec!["topic-ownership".to_string()],
                estimated_hours: 2.0,
            },
            RoadmapTopic {
                id: "topic-lifetimes".to_string(),
                title: "Lifetime Annotations & Elision Rules".to_string(),
                description: "Generic lifetime bounds, struct references, subtyping, and 'static constraints.".to_string(),
                difficulty: 0.6,
                prerequisites: vec!["topic-borrowing".to_string()],
                estimated_hours: 4.0,
            },
            RoadmapTopic {
                id: "topic-traits".to_string(),
                title: "Trait Bounds, Generics & Object Safety".to_string(),
                description: "Monomorphization, dynamic dispatch (dyn Trait), supertraits, and auto traits (Send/Sync).".to_string(),
                difficulty: 0.55,
                prerequisites: vec!["topic-borrowing".to_string()],
                estimated_hours: 3.5,
            },
            RoadmapTopic {
                id: "topic-tokio".to_string(),
                title: "Tokio Concurrency & Async Channels".to_string(),
                description: "Cooperative task scheduling, async/await runtime, MPSC messaging, and state synchronization.".to_string(),
                difficulty: 0.75,
                prerequisites: vec!["topic-traits".to_string(), "topic-lifetimes".to_string()],
                estimated_hours: 5.0,
            },
            RoadmapTopic {
                id: "topic-pin-futures".to_string(),
                title: "Pinning, Custom Futures & Async Streams".to_string(),
                description: "Pin projection, Unpin invariants, manual Future::poll implementation, and Stream combinators.".to_string(),
                difficulty: 0.85,
                prerequisites: vec!["topic-tokio".to_string()],
                estimated_hours: 6.0,
            },
            RoadmapTopic {
                id: "topic-unsafe".to_string(),
                title: "Unsafe Rust & FFI Invariants".to_string(),
                description: "Raw pointers, undefined behavior avoidance, sound encapsulated abstractions, and C FFI.".to_string(),
                difficulty: 0.9,
                prerequisites: vec!["topic-lifetimes".to_string()],
                estimated_hours: 5.0,
            },
        ]
    }

    /// Access the curriculum catalog.
    pub fn default_catalog(&self) -> &[RoadmapTopic] {
        &self.default_catalog
    }

    /// Calculate a weakness priority weight for a topic given the learner's weakness report.
    ///
    /// Topics addressing recommended topics receive top priority boost (+100.0).
    /// Topics matching weak area keywords receive proportional weight (+50.0).
    pub fn calculate_relevance_score(topic: &RoadmapTopic, weakness: &WeaknessReport) -> f32 {
        let mut score = 0.0;
        let title_lower = topic.title.to_lowercase();
        let desc_lower = topic.description.to_lowercase();
        let id_lower = topic.id.to_lowercase();

        // 1. Exact or partial match with recommended topics
        for rec in &weakness.recommended_topics {
            let rec_lower = rec.to_lowercase();
            if title_lower.contains(&rec_lower)
                || rec_lower.contains(&title_lower)
                || id_lower.contains(&rec_lower)
            {
                score += 100.0;
            }
        }

        // 2. Keyword matching with weak areas and severity scaling
        for weak in &weakness.weak_areas {
            let weak_lower = weak.to_lowercase();
            let multiplier = if weak_lower.contains("critical") {
                3.0
            } else if weak_lower.contains("high") {
                2.0
            } else if weak_lower.contains("medium") {
                1.5
            } else {
                1.0
            };

            let keywords: &[&str] = if weak_lower.contains("ownership") {
                &["ownership", "move", "raii", "drop"]
            } else if weak_lower.contains("borrow") {
                &["borrow", "mutab", "reference", "aliasing"]
            } else if weak_lower.contains("lifetime") {
                &["lifetime", "elision", "'static"]
            } else if weak_lower.contains("tokio") || weak_lower.contains("concurrency") {
                &["tokio", "async", "concurrency", "channel", "mpsc"]
            } else if weak_lower.contains("error") {
                &["error", "thiserror", "result"]
            } else if weak_lower.contains("trait") {
                &["trait", "bound", "generic", "dispatch"]
            } else {
                &[]
            };

            for &kw in keywords {
                if title_lower.contains(kw) || desc_lower.contains(kw) {
                    score += 50.0 * multiplier;
                    break;
                }
            }
        }

        score
    }

    /// Pure Directed Acyclic Graph (DAG) topological sort of topics based on prerequisites.
    ///
    /// Implements Kahn's algorithm. Returns `Err(CoachError::CycleDetected)` if a circular
    /// prerequisite loop is present.
    pub fn topological_sort(
        &self,
        topics: &[RoadmapTopic],
    ) -> Result<Vec<RoadmapTopic>, CoachError> {
        let dummy_weakness = WeaknessReport::default();
        self.prioritized_topological_sort(topics, &dummy_weakness)
    }

    /// Directed Acyclic Graph (DAG) topological sort with dynamic reordering based on WeaknessReport.
    ///
    /// At each step where multiple topics have satisfied prerequisites (in-degree == 0),
    /// the topic with the highest weakness relevance score is chosen first.
    /// Prerequisite ordering is strictly preserved.
    pub fn prioritized_topological_sort(
        &self,
        topics: &[RoadmapTopic],
        weakness: &WeaknessReport,
    ) -> Result<Vec<RoadmapTopic>, CoachError> {
        if topics.is_empty() {
            return Ok(Vec::new());
        }

        // Map topic ID -> RoadmapTopic
        let topic_map: HashMap<String, RoadmapTopic> =
            topics.iter().map(|t| (t.id.clone(), t.clone())).collect();

        let topic_ids: HashSet<String> = topic_map.keys().cloned().collect();

        // Build in-degree map and adjacency list (prereq -> dependents)
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        for topic in topics {
            in_degree.entry(topic.id.clone()).or_insert(0);
            dependents.entry(topic.id.clone()).or_default();
        }

        for topic in topics {
            for prereq in &topic.prerequisites {
                if topic_ids.contains(prereq) {
                    *in_degree.entry(topic.id.clone()).or_insert(0) += 1;
                    dependents
                        .entry(prereq.clone())
                        .or_default()
                        .push(topic.id.clone());
                }
            }
        }

        // Available candidate queue (in-degree == 0)
        let mut ready: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut sorted = Vec::with_capacity(topics.len());

        while !ready.is_empty() {
            // Sort ready queue:
            // 1. Highest weakness relevance score first
            // 2. Lowest difficulty first (smoother pedagogical onboarding)
            // 3. Topic ID ascending for deterministic tie-breaking
            ready.sort_by(|a_id, b_id| {
                let topic_a = &topic_map[a_id];
                let topic_b = &topic_map[b_id];

                let score_a = Self::calculate_relevance_score(topic_a, weakness);
                let score_b = Self::calculate_relevance_score(topic_b, weakness);

                // Priority: higher score first
                match score_b.partial_cmp(&score_a).unwrap_or(Ordering::Equal) {
                    Ordering::Equal => {
                        // Easier difficulty first
                        match topic_a
                            .difficulty
                            .partial_cmp(&topic_b.difficulty)
                            .unwrap_or(Ordering::Equal)
                        {
                            Ordering::Equal => a_id.cmp(b_id),
                            diff_order => diff_order,
                        }
                    }
                    score_order => score_order,
                }
            });

            let next_id = ready.remove(0);
            let next_topic = topic_map[&next_id].clone();
            sorted.push(next_topic);

            // Decrement in-degree for all direct dependents
            if let Some(deps) = dependents.get(&next_id) {
                for dep_id in deps {
                    if let Some(deg) = in_degree.get_mut(dep_id) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            ready.push(dep_id.clone());
                        }
                    }
                }
            }
        }

        // Check for cycles
        if sorted.len() < topics.len() {
            let visited: HashSet<String> = sorted.iter().map(|t| t.id.clone()).collect();
            let cyclic_topics: Vec<String> = topics
                .iter()
                .filter(|t| !visited.contains(&t.id))
                .map(|t| t.id.clone())
                .collect();
            return Err(CoachError::CycleDetected(cyclic_topics));
        }

        Ok(sorted)
    }

    /// Estimate milestone completion dates and roadmap feasibility.
    pub fn calculate_schedule(&self, roadmap: &Roadmap, profile: &UserProfile) -> RoadmapSchedule {
        self.calculate_schedule_from_start(roadmap, profile, Utc::now())
    }

    /// Estimate milestone schedule starting from an explicit base timestamp.
    pub fn calculate_schedule_from_start(
        &self,
        roadmap: &Roadmap,
        profile: &UserProfile,
        start_time: DateTime<Utc>,
    ) -> RoadmapSchedule {
        let hours_per_day = profile.hours_per_day.max(0.0);
        let deadline_days = profile.deadline_days;

        let mut cumulative_hours = 0.0;
        let mut milestones = Vec::with_capacity(roadmap.topics.len());

        for topic in &roadmap.topics {
            cumulative_hours += topic.estimated_hours;

            let target_day = if hours_per_day > 0.0 {
                cumulative_hours / hours_per_day
            } else {
                0.0
            };

            let days_offset_ms = (target_day * 24.0 * 3600.0 * 1000.0) as i64;
            let target_date = start_time + Duration::milliseconds(days_offset_ms);
            let within_deadline = target_day <= deadline_days as f32;

            milestones.push(MilestoneSchedule {
                topic_id: topic.id.clone(),
                topic_title: topic.title.clone(),
                estimated_hours: topic.estimated_hours,
                cumulative_hours,
                target_day,
                target_date,
                within_deadline,
            });
        }

        let total_hours = cumulative_hours;
        let total_days = if hours_per_day > 0.0 {
            total_hours / hours_per_day
        } else {
            0.0
        };

        let required_hours_per_day = if deadline_days > 0 {
            total_hours / deadline_days as f32
        } else {
            0.0
        };

        let feasible = hours_per_day > 0.0 && total_days <= deadline_days as f32;

        let pace_warning = if hours_per_day <= 0.0 {
            Some("Daily study hours must be greater than zero to estimate schedule.".to_string())
        } else if !feasible {
            Some(format!(
                "Current pace ({:.1}h/day) requires {:.1} days, exceeding the {}-day deadline. Adjust study time to at least {:.1}h/day.",
                hours_per_day, total_days, deadline_days, required_hours_per_day
            ))
        } else {
            None
        };

        RoadmapSchedule {
            milestones,
            total_hours,
            total_days,
            deadline_days,
            hours_per_day,
            required_hours_per_day,
            feasible,
            pace_warning,
        }
    }

    /// Compile a customized, weakness-prioritized learning roadmap for the student.
    pub fn generate_roadmap(&self, profile: &UserProfile, weakness: &WeaknessReport) -> Roadmap {
        // Filter catalog based on target level
        let candidates: Vec<RoadmapTopic> = match profile.target_level {
            UserLevel::Beginner => self
                .default_catalog
                .iter()
                .filter(|t| t.difficulty <= 0.5)
                .cloned()
                .collect(),
            UserLevel::Intermediate => self
                .default_catalog
                .iter()
                .filter(|t| t.difficulty <= 0.8)
                .cloned()
                .collect(),
            UserLevel::Advanced => self.default_catalog.clone(),
        };

        let sorted_topics = self
            .prioritized_topological_sort(&candidates, weakness)
            .unwrap_or(candidates);

        Roadmap {
            topics: sorted_topics,
        }
    }

    /// Compile a customized learning roadmap with prepended micro-drills for decayed concepts.
    pub fn generate_roadmap_with_decay(
        &self,
        profile: &UserProfile,
        weakness: &WeaknessReport,
        decayed_drills: Vec<RoadmapTopic>,
    ) -> Roadmap {
        let mut roadmap = self.generate_roadmap(profile, weakness);
        self.inject_decayed_concept_drills(&mut roadmap, decayed_drills);
        roadmap
    }

    /// Prepends micro-drills for decayed concepts to the front of a roadmap.
    pub fn inject_decayed_concept_drills(
        &self,
        roadmap: &mut Roadmap,
        decayed_drills: Vec<RoadmapTopic>,
    ) {
        if !decayed_drills.is_empty() {
            let mut combined = decayed_drills;
            combined.append(&mut roadmap.topics);
            roadmap.topics = combined;
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
