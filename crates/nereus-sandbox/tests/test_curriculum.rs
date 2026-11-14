//! Comprehensive test suite for the curated curriculum bank and automated test harnesses.
//!
//! Verifies:
//! 1. Curriculum metadata, challenge discovery, and default roadmap topics.
//! 2. Reference solutions successfully passing all hidden test harness assertions.
//! 3. Buggy / incorrect solutions failing test harness assertions or compiler checks.

use nereus_core::traits::SandboxRunner;
use nereus_sandbox::curriculum::Curriculum;
use nereus_sandbox::host::HostSandboxRunner;
use std::time::Duration;

fn make_runner() -> HostSandboxRunner {
    HostSandboxRunner::new(Duration::from_secs(10))
}

// ============================================================================
// 1. Curriculum Metadata & Discovery Tests
// ============================================================================

#[test]
fn test_curriculum_contains_four_core_challenges() {
    let challenges = Curriculum::challenges();
    assert_eq!(
        challenges.len(),
        4,
        "Curriculum must contain exactly 4 starter challenges"
    );

    let ids: Vec<&str> = challenges.iter().map(|c| c.id.as_str()).collect();
    assert!(ids.contains(&"ownership-move"));
    assert!(ids.contains(&"concurrency-arc-mutex"));
    assert!(ids.contains(&"async-mpsc-channels"));
    assert!(ids.contains(&"traits-generics"));
}

#[test]
fn test_curriculum_challenge_properties_valid() {
    for c in Curriculum::challenges() {
        assert!(!c.id.is_empty(), "Challenge ID must not be empty");
        assert!(!c.title.is_empty(), "Challenge title must not be empty");
        assert!(
            !c.description.is_empty(),
            "Challenge description must not be empty"
        );
        assert!(
            !c.starter_code.is_empty(),
            "Challenge starter_code must not be empty"
        );
        assert!(
            !c.test_harness_code.is_empty(),
            "Challenge test_harness_code must not be empty"
        );
        assert!(
            c.difficulty > 0.0 && c.difficulty <= 1.0,
            "Difficulty must be in (0.0, 1.0]"
        );
        assert!(
            c.test_harness_code.contains("#[test]")
                || c.test_harness_code.contains("#[tokio::test]")
        );
    }
}

#[test]
fn test_curriculum_get_challenge_direct_and_aliases() {
    // Direct ID lookup
    assert!(Curriculum::get_challenge("ownership-move").is_some());
    assert!(Curriculum::get_challenge("concurrency-arc-mutex").is_some());
    assert!(Curriculum::get_challenge("async-mpsc-channels").is_some());
    assert!(Curriculum::get_challenge("traits-generics").is_some());

    // Known topic aliases
    assert_eq!(
        Curriculum::get_challenge("topic-ownership").unwrap().id,
        "ownership-move"
    );
    assert_eq!(
        Curriculum::get_challenge("topic-traits").unwrap().id,
        "traits-generics"
    );
    assert_eq!(
        Curriculum::get_challenge("topic-concurrency").unwrap().id,
        "concurrency-arc-mutex"
    );
    assert_eq!(
        Curriculum::get_challenge("topic-tokio").unwrap().id,
        "async-mpsc-channels"
    );

    // Non-existent challenge
    assert!(Curriculum::get_challenge("non-existent-topic-404").is_none());
}

#[test]
fn test_curriculum_default_topics_conversion() {
    let topics = Curriculum::default_topics();
    assert_eq!(topics.len(), 4);

    let topic_ids: Vec<&str> = topics.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(
        topic_ids,
        vec![
            "ownership-move",
            "concurrency-arc-mutex",
            "async-mpsc-channels",
            "traits-generics"
        ]
    );

    for topic in &topics {
        assert!(topic.estimated_hours > 0.0);
        assert!(topic.difficulty > 0.0 && topic.difficulty <= 1.0);
    }
}

// ============================================================================
// 2. Challenge 1: ownership-move Tests
// ============================================================================

#[tokio::test]
async fn test_challenge_ownership_move_reference_solution_passes() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("ownership-move").unwrap();
    let topic = challenge.to_roadmap_topic();

    let correct_code = r#"
pub fn transfer_and_count(s: String) -> (String, usize) {
    let count = s.chars().count();
    (s, count)
}
"#;

    let result = runner.execute(&topic, correct_code).await.unwrap();
    assert!(
        result.success,
        "Reference solution for ownership-move must pass: exit={}, stdout='{}', stderr='{}'",
        result.exit_code, result.stdout, result.stderr
    );
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("test result: ok") || result.stdout.contains("passed"));
}

#[tokio::test]
async fn test_challenge_ownership_move_incorrect_count_fails() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("ownership-move").unwrap();
    let topic = challenge.to_roadmap_topic();

    // Returns incorrect character count
    let buggy_code = r#"
pub fn transfer_and_count(s: String) -> (String, usize) {
    let wrong_count = s.chars().count() + 42;
    (s, wrong_count)
}
"#;

    let result = runner.execute(&topic, buggy_code).await.unwrap();
    assert!(
        !result.success,
        "Buggy solution with incorrect count must fail"
    );
    assert_ne!(result.exit_code, 0);
}

#[tokio::test]
async fn test_challenge_ownership_move_compiler_error_fails() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("ownership-move").unwrap();
    let topic = challenge.to_roadmap_topic();

    // Violates move semantics: drops `s` then attempts to return it
    let invalid_code = r#"
pub fn transfer_and_count(s: String) -> (String, usize) {
    drop(s);
    (s, 0)
}
"#;

    let (result, diags) = runner
        .execute_and_diagnose(&topic, invalid_code)
        .await
        .unwrap();
    assert!(
        !result.success,
        "Code violating ownership must fail compilation"
    );
    assert_ne!(result.exit_code, 0);
    assert!(result.stderr.contains("E0382") || result.stderr.contains("use of moved value"));
    assert!(!diags.is_empty());
}

// ============================================================================
// 3. Challenge 2: concurrency-arc-mutex Tests
// ============================================================================

#[tokio::test]
async fn test_challenge_concurrency_arc_mutex_reference_solution_passes() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("concurrency-arc-mutex").unwrap();
    let topic = challenge.to_roadmap_topic();

    let correct_code = r#"
use std::sync::{Arc, Mutex};
use std::thread;

pub fn run_worker_queue(num_workers: usize, items_per_worker: usize) -> Arc<Mutex<Vec<u32>>> {
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::with_capacity(num_workers);

    for i in 0..num_workers {
        let shared_clone = Arc::clone(&shared);
        let handle = thread::spawn(move || {
            for j in 0..items_per_worker {
                let val = (i * 1000 + j) as u32;
                let mut guard = shared_clone.lock().unwrap();
                guard.push(val);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    shared
}
"#;

    let result = runner.execute(&topic, correct_code).await.unwrap();
    assert!(
        result.success,
        "Reference solution for concurrency-arc-mutex must pass"
    );
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("test result: ok") || result.stdout.contains("passed"));
}

#[tokio::test]
async fn test_challenge_concurrency_arc_mutex_empty_queue_fails() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("concurrency-arc-mutex").unwrap();
    let topic = challenge.to_roadmap_topic();

    // Returns empty state without spawning workers
    let buggy_code = r#"
use std::sync::{Arc, Mutex};

pub fn run_worker_queue(_num_workers: usize, _items_per_worker: usize) -> Arc<Mutex<Vec<u32>>> {
    Arc::new(Mutex::new(Vec::new()))
}
"#;

    let result = runner.execute(&topic, buggy_code).await.unwrap();
    assert!(
        !result.success,
        "Empty worker queue must fail multi-worker test"
    );
    assert_ne!(result.exit_code, 0);
}

#[tokio::test]
async fn test_challenge_concurrency_arc_mutex_compiler_error_fails() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("concurrency-arc-mutex").unwrap();
    let topic = challenge.to_roadmap_topic();

    // Fails to clone Arc before moving into thread closure
    let invalid_code = r#"
use std::sync::{Arc, Mutex};
use std::thread;

pub fn run_worker_queue(num_workers: usize, _items_per_worker: usize) -> Arc<Mutex<Vec<u32>>> {
    let shared = Arc::new(Mutex::new(Vec::new()));
    for _ in 0..num_workers {
        thread::spawn(move || {
            let mut guard = shared.lock().unwrap();
            guard.push(1);
        });
    }
    shared
}
"#;

    let (result, diags) = runner
        .execute_and_diagnose(&topic, invalid_code)
        .await
        .unwrap();
    assert!(
        !result.success,
        "Moving un-cloned Arc into closure must fail compilation"
    );
    assert_ne!(result.exit_code, 0);
    assert!(result.stderr.contains("E0382") || result.stderr.contains("use of moved value"));
    assert!(!diags.is_empty());
}

// ============================================================================
// 4. Challenge 3: async-mpsc-channels Tests
// ============================================================================

#[tokio::test]
async fn test_challenge_async_mpsc_reference_solution_passes() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("async-mpsc-channels").unwrap();
    let topic = challenge.to_roadmap_topic();

    let correct_code = r#"
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[derive(Debug, PartialEq, Eq)]
pub enum ChannelError {
    Timeout,
    Closed,
}

pub async fn consume_with_timeout<T>(
    rx: &mut mpsc::Receiver<T>,
    max_items: usize,
    timeout_duration: Duration,
) -> Result<Vec<T>, ChannelError> {
    let mut items = Vec::with_capacity(max_items);

    while items.len() < max_items {
        match timeout(timeout_duration, rx.recv()).await {
            Ok(Some(item)) => items.push(item),
            Ok(None) => return Err(ChannelError::Closed),
            Err(_) => return Err(ChannelError::Timeout),
        }
    }

    Ok(items)
}
"#;

    let result = runner.execute(&topic, correct_code).await.unwrap();
    assert!(
        result.success,
        "Reference solution for async-mpsc-channels must pass: stdout='{}', stderr='{}'",
        result.stdout, result.stderr
    );
    assert_eq!(result.exit_code, 0);
}

#[tokio::test]
async fn test_challenge_async_mpsc_immediate_error_fails() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("async-mpsc-channels").unwrap();
    let topic = challenge.to_roadmap_topic();

    // Always returns closed error immediately
    let buggy_code = r#"
use tokio::sync::mpsc;
use tokio::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum ChannelError {
    Timeout,
    Closed,
}

pub async fn consume_with_timeout<T>(
    _rx: &mut mpsc::Receiver<T>,
    _max_items: usize,
    _timeout_duration: Duration,
) -> Result<Vec<T>, ChannelError> {
    Err(ChannelError::Closed)
}
"#;

    let result = runner.execute(&topic, buggy_code).await.unwrap();
    assert!(
        !result.success,
        "Always-failing consumer must fail unit test assertions"
    );
    assert_ne!(result.exit_code, 0);
}

// ============================================================================
// 5. Challenge 4: traits-generics Tests
// ============================================================================

#[tokio::test]
async fn test_challenge_traits_generics_reference_solution_passes() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("traits-generics").unwrap();
    let topic = challenge.to_roadmap_topic();

    let correct_code = r#"
pub trait Evaluator {
    fn evaluate(&self) -> f64;
    fn name(&self) -> &str;
}

pub fn total_score_static<T: Evaluator>(items: &[T]) -> f64 {
    items.iter().map(|item| item.evaluate()).sum()
}

pub fn total_score_dynamic(items: &[&dyn Evaluator]) -> f64 {
    items.iter().map(|item| item.evaluate()).sum()
}

pub fn top_evaluator_name<'a>(items: &[&'a dyn Evaluator]) -> Option<&'a str> {
    items
        .iter()
        .max_by(|a, b| a.evaluate().partial_cmp(&b.evaluate()).unwrap_or(std::cmp::Ordering::Equal))
        .map(|item| item.name())
}
"#;

    let result = runner.execute(&topic, correct_code).await.unwrap();
    assert!(
        result.success,
        "Reference solution for traits-generics must pass"
    );
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("test result: ok") || result.stdout.contains("passed"));
}

#[tokio::test]
async fn test_challenge_traits_generics_stub_implementation_fails() {
    let runner = make_runner();
    let challenge = Curriculum::get_challenge("traits-generics").unwrap();
    let topic = challenge.to_roadmap_topic();

    // Returns dummy constant values that fail assertions
    let buggy_code = r#"
pub trait Evaluator {
    fn evaluate(&self) -> f64;
    fn name(&self) -> &str;
}

pub fn total_score_static<T: Evaluator>(_items: &[T]) -> f64 {
    0.0
}

pub fn total_score_dynamic(_items: &[&dyn Evaluator]) -> f64 {
    0.0
}

pub fn top_evaluator_name<'a>(_items: &[&'a dyn Evaluator]) -> Option<&'a str> {
    None
}
"#;

    let result = runner.execute(&topic, buggy_code).await.unwrap();
    assert!(
        !result.success,
        "Stub implementation returning 0.0 must fail assertions"
    );
    assert_ne!(result.exit_code, 0);
}
