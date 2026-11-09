//! Curated starter curriculum and verification test harnesses.
//!
//! Provides high-yield Rust challenges with hidden automated test suites
//! for ownership, concurrency, async messaging, and traits/generics.

use nereus_core::models::RoadmapTopic;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Represents a single hands-on programming challenge in the curriculum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub title: String,
    pub description: String,
    pub starter_code: String,
    pub test_harness_code: String,
    pub difficulty: f32,
    pub prerequisites: Vec<String>,
}

impl Challenge {
    /// Converts this curriculum challenge into a standard `RoadmapTopic`.
    pub fn to_roadmap_topic(&self) -> RoadmapTopic {
        let estimated_hours = match self.id.as_str() {
            "ownership-move" => 2.0,
            "traits-generics" => 2.5,
            "concurrency-arc-mutex" => 3.0,
            "async-mpsc-channels" => 3.5,
            _ => (self.difficulty * 4.0).max(1.0),
        };

        RoadmapTopic {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            difficulty: self.difficulty,
            prerequisites: self.prerequisites.clone(),
            estimated_hours,
        }
    }
}

/// Central catalog and discovery interface for curated learning challenges.
pub struct Curriculum;

impl Curriculum {
    /// Returns the static slice of all curated starter challenges.
    pub fn challenges() -> &'static [Challenge] {
        static CHALLENGES: OnceLock<Vec<Challenge>> = OnceLock::new();
        CHALLENGES.get_or_init(|| {
            vec![
                // 1. Ownership & Move Semantics
                Challenge {
                    id: "ownership-move".to_string(),
                    title: "Ownership & Move Semantics".to_string(),
                    description: "Understand affine types and move semantics. Implement transfer_and_count(s: String) -> (String, usize) to take ownership of a String, compute its character count, and return both the original String and count as a tuple.".to_string(),
                    starter_code: r#"/// Takes ownership of a String, calculates its character count,
/// and returns a tuple containing the original String and its character count.
pub fn transfer_and_count(s: String) -> (String, usize) {
    // TODO: Implement ownership transfer and character counting
    todo!()
}
"#.to_string(),
                    test_harness_code: r#"#[cfg(test)]
mod curriculum_ownership_tests {
    use super::*;

    #[test]
    fn test_transfer_and_count_basic() {
        let input = String::from("hello world");
        let (returned_str, count) = transfer_and_count(input);
        assert_eq!(returned_str, "hello world");
        assert_eq!(count, 11);
    }

    #[test]
    fn test_transfer_and_count_empty() {
        let input = String::new();
        let (returned_str, count) = transfer_and_count(input);
        assert_eq!(returned_str, "");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_transfer_and_count_unicode() {
        let input = String::from("🦀 rustaceans 🦀");
        let (returned_str, count) = transfer_and_count(input);
        assert_eq!(returned_str, "🦀 rustaceans 🦀");
        assert_eq!(count, 14);
    }

    #[test]
    fn test_transfer_preserves_capacity_and_ownership() {
        let mut input = String::with_capacity(64);
        input.push_str("ownership test");
        let (returned_str, count) = transfer_and_count(input);
        assert_eq!(returned_str, "ownership test");
        assert_eq!(count, 14);
        assert!(returned_str.capacity() >= 64);
    }
}
"#.to_string(),
                    difficulty: 0.3,
                    prerequisites: vec![],
                },

                // 2. Thread-Safe Concurrency with Arc and Mutex
                Challenge {
                    id: "concurrency-arc-mutex".to_string(),
                    title: "Thread-Safe Concurrency with Arc and Mutex".to_string(),
                    description: "Implement multi-threaded shared state using Arc<Mutex<Vec<u32>>>. Spawn worker threads pushing numbers into the shared vector and join them before returning.".to_string(),
                    starter_code: r#"use std::sync::{Arc, Mutex};
use std::thread;

/// Spawns `num_workers` threads. Each worker pushes numbers into a shared `Arc<Mutex<Vec<u32>>>`.
/// Worker `i` (0..num_workers) pushes `items_per_worker` items: `(i * 1000 + j) as u32` for `j in 0..items_per_worker`.
/// Joins all threads before returning the shared vector.
pub fn run_worker_queue(num_workers: usize, items_per_worker: usize) -> Arc<Mutex<Vec<u32>>> {
    // TODO: Initialize Arc<Mutex<Vec<u32>>>, spawn worker threads, push items, join, and return
    todo!()
}
"#.to_string(),
                    test_harness_code: r#"#[cfg(test)]
mod curriculum_concurrency_tests {
    use super::*;

    #[test]
    fn test_worker_queue_zero_workers() {
        let shared = run_worker_queue(0, 5);
        let data = shared.lock().unwrap();
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_worker_queue_single_worker() {
        let shared = run_worker_queue(1, 10);
        let data = shared.lock().unwrap();
        assert_eq!(data.len(), 10);
        for (idx, &val) in data.iter().enumerate() {
            assert_eq!(val, idx as u32);
        }
    }

    #[test]
    fn test_worker_queue_multi_threaded_accumulation() {
        let num_workers = 4;
        let items_per_worker = 25;
        let shared = run_worker_queue(num_workers, items_per_worker);
        let mut data = shared.lock().unwrap().clone();
        assert_eq!(data.len(), num_workers * items_per_worker);
        data.sort();

        let mut expected = Vec::new();
        for i in 0..num_workers {
            for j in 0..items_per_worker {
                expected.push((i * 1000 + j) as u32);
            }
        }
        expected.sort();
        assert_eq!(data, expected);
    }
}
"#.to_string(),
                    difficulty: 0.6,
                    prerequisites: vec!["ownership-move".to_string()],
                },

                // 3. Tokio Async MPSC Channels
                Challenge {
                    id: "async-mpsc-channels".to_string(),
                    title: "Tokio Async MPSC Channels & Timeouts".to_string(),
                    description: "Tokio async MPSC producer/consumer channel with timeout handling. Implement a consumer that returns an error when reading times out or channel closes.".to_string(),
                    starter_code: r#"use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[derive(Debug, PartialEq, Eq)]
pub enum ChannelError {
    Timeout,
    Closed,
}

/// Consumes up to `max_items` from the receiver `rx`.
/// Each item receive operation is guarded by `timeout_duration`.
/// If receiving an item times out, returns `Err(ChannelError::Timeout)`.
/// If the channel is closed before reaching `max_items`, returns `Err(ChannelError::Closed)`.
/// If all `max_items` are successfully received, returns `Ok(Vec<T>)`.
pub async fn consume_with_timeout<T>(
    rx: &mut mpsc::Receiver<T>,
    max_items: usize,
    timeout_duration: Duration,
) -> Result<Vec<T>, ChannelError> {
    // TODO: Implement timed consumer loop
    todo!()
}
"#.to_string(),
                    test_harness_code: r#"#[cfg(test)]
mod curriculum_async_tests {
    use super::*;

    #[tokio::test]
    async fn test_consume_all_items_successfully() {
        let (tx, mut rx) = mpsc::channel(10);
        tokio::spawn(async move {
            for i in 1..=5 {
                tx.send(i).await.unwrap();
            }
        });

        let result = consume_with_timeout(&mut rx, 5, Duration::from_millis(100)).await;
        assert_eq!(result, Ok(vec![1, 2, 3, 4, 5]));
    }

    #[tokio::test]
    async fn test_consume_times_out_when_producer_delayed() {
        let (tx, mut rx) = mpsc::channel(10);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let _ = tx.send(42).await;
        });

        let result = consume_with_timeout(&mut rx, 1, Duration::from_millis(50)).await;
        assert_eq!(result, Err(ChannelError::Timeout));
    }

    #[tokio::test]
    async fn test_consume_channel_closed_prematurely() {
        let (tx, mut rx) = mpsc::channel::<i32>(10);
        drop(tx);

        let result = consume_with_timeout(&mut rx, 1, Duration::from_millis(100)).await;
        assert_eq!(result, Err(ChannelError::Closed));
    }
}
"#.to_string(),
                    difficulty: 0.75,
                    prerequisites: vec!["concurrency-arc-mutex".to_string()],
                },

                // 4. Traits and Generics
                Challenge {
                    id: "traits-generics".to_string(),
                    title: "Traits, Generic Bounds & Dynamic Dispatch".to_string(),
                    description: "Custom trait definition with static dispatch via generic bounds and dynamic dispatch via trait objects.".to_string(),
                    starter_code: r#"/// Trait representing an evaluatable entity that yields a numerical score and name.
pub trait Evaluator {
    fn evaluate(&self) -> f64;
    fn name(&self) -> &str;
}

/// Computes the sum of all scores using static dispatch (generic bounds).
pub fn total_score_static<T: Evaluator>(items: &[T]) -> f64 {
    // TODO: Sum evaluate() results using generic bounds
    todo!()
}

/// Computes the sum of all scores using dynamic dispatch (trait objects).
pub fn total_score_dynamic(items: &[&dyn Evaluator]) -> f64 {
    // TODO: Sum evaluate() results across heterogeneous trait objects
    todo!()
}

/// Returns the name of the highest scoring evaluator using dynamic dispatch.
/// Returns None if the slice is empty.
pub fn top_evaluator_name<'a>(items: &[&'a dyn Evaluator]) -> Option<&'a str> {
    // TODO: Find evaluator with maximum score and return its name
    todo!()
}
"#.to_string(),
                    test_harness_code: r#"#[cfg(test)]
mod curriculum_trait_tests {
    use super::*;

    struct Quiz {
        title: String,
        score: f64,
    }

    impl Evaluator for Quiz {
        fn evaluate(&self) -> f64 {
            self.score
        }
        fn name(&self) -> &str {
            &self.title
        }
    }

    struct Project {
        repo: &'static str,
        grade: f64,
    }

    impl Evaluator for Project {
        fn evaluate(&self) -> f64 {
            self.grade
        }
        fn name(&self) -> &str {
            self.repo
        }
    }

    #[test]
    fn test_static_dispatch() {
        let quizzes = vec![
            Quiz { title: "Ownership".to_string(), score: 85.0 },
            Quiz { title: "Lifetimes".to_string(), score: 95.0 },
        ];
        assert_eq!(total_score_static(&quizzes), 180.0);
    }

    #[test]
    fn test_dynamic_dispatch() {
        let q = Quiz { title: "Concurrency".to_string(), score: 90.0 };
        let p = Project { repo: "nereus-core", grade: 98.0 };
        let items: Vec<&dyn Evaluator> = vec![&q, &p];
        assert_eq!(total_score_dynamic(&items), 188.0);
    }

    #[test]
    fn test_top_evaluator() {
        let q = Quiz { title: "Concurrency".to_string(), score: 90.0 };
        let p = Project { repo: "nereus-core", grade: 98.0 };
        let items: Vec<&dyn Evaluator> = vec![&q, &p];
        assert_eq!(top_evaluator_name(&items), Some("nereus-core"));
        assert_eq!(top_evaluator_name(&[]), None);
    }
}
"#.to_string(),
                    difficulty: 0.5,
                    prerequisites: vec!["ownership-move".to_string()],
                },
            ]
        })
    }

    /// Looks up a challenge by topic ID or canonical alias.
    pub fn get_challenge(topic_id: &str) -> Option<&'static Challenge> {
        let trimmed = topic_id.trim();
        let challenges = Self::challenges();

        // 1. Direct exact ID match
        if let Some(c) = challenges.iter().find(|c| c.id == trimmed) {
            return Some(c);
        }

        // 2. Exact match on known aliases/topic IDs
        let canonical_id = match trimmed {
            "topic-ownership" | "ownership" => Some("ownership-move"),
            "topic-traits" | "traits" => Some("traits-generics"),
            "topic-concurrency" | "concurrency" => Some("concurrency-arc-mutex"),
            "topic-tokio" | "tokio" | "mpsc" => Some("async-mpsc-channels"),
            _ => None,
        };

        if let Some(target) = canonical_id {
            if let Some(c) = challenges.iter().find(|c| c.id == target) {
                return Some(c);
            }
        }

        // 3. Substring match fallback
        challenges
            .iter()
            .find(|c| trimmed.contains(&c.id) || c.id.contains(trimmed))
    }

    /// Returns the default roadmap topics generated from the curated challenge bank.
    pub fn default_topics() -> Vec<RoadmapTopic> {
        Self::challenges()
            .iter()
            .map(|c| c.to_roadmap_topic())
            .collect()
    }
}
