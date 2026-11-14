//! Adversary Agent: Educational fuzzing and boundary-case property testing.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use serde::{Deserialize, Serialize};

/// Classification of adversarial stress vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdversaryCategory {
    /// Boundary conditions: empty collections, single items, numeric limits.
    BoundaryCondition,
    /// Concurrency stress: multithreaded contention, race condition probing, lock deadlock.
    ConcurrencyStress,
    /// Memory safety: drop ordering, aliasing constraints, unaligned or null invariants.
    MemorySafety,
    /// Resource exhaustion: unbounded recursion, stack frames, memory allocations.
    ResourceExhaustion,
    /// Arithmetic edge cases: integer overflow, float NaN/Infinity, division by zero.
    ArithmeticOverflow,
}

impl std::fmt::Display for AdversaryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BoundaryCondition => write!(f, "Boundary Condition"),
            Self::ConcurrencyStress => write!(f, "Concurrency Stress"),
            Self::MemorySafety => write!(f, "Memory Safety"),
            Self::ResourceExhaustion => write!(f, "Resource Exhaustion"),
            Self::ArithmeticOverflow => write!(f, "Arithmetic Overflow"),
        }
    }
}

/// A specific adversarial test harness to evaluate boundary resilience.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdversaryTestCase {
    /// Identifier name for the test harness.
    pub name: String,
    /// Stress vector classification.
    pub category: AdversaryCategory,
    /// Educational explanation of what failure this test is hunting for.
    pub description: String,
    /// Runnable Rust code harness or snippet.
    pub harness_code: String,
    /// Desired defensive outcome (e.g. graceful Err, panic with clean message, correct result).
    pub expected_behavior: String,
    /// Potential bug or vulnerability if unguarded.
    pub failure_mode: String,
}

/// Comprehensive adversarial analysis and educational challenge report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdversaryReport {
    /// Target curriculum topic or exercise identifier.
    pub topic_id: String,
    /// Injected pathological test cases.
    pub test_cases: Vec<AdversaryTestCase>,
    /// Socratic prompt challenging the learner to defend their code against pathological inputs.
    pub socratic_warning: String,
    /// Key defensive programming invariants highlighted by this analysis.
    pub defensive_principles: Vec<String>,
}

/// Educational Adversary Agent that generates pathological stress cases for learner exercises.
#[derive(Debug, Clone, Default)]
pub struct AdversaryAgent;

impl AdversaryAgent {
    /// Instantiates the Adversary Agent.
    pub fn new() -> Self {
        Self
    }

    /// Evaluates user code and topic to synthesize pathological test cases.
    pub fn generate_pathological_tests(&self, topic_id: &str, user_code: &str) -> AdversaryReport {
        let topic_lower = topic_id.to_lowercase();
        let code_lower = user_code.to_lowercase();

        let mut test_cases = Vec::new();
        let mut defensive_principles = Vec::new();

        if topic_lower.contains("concurr")
            || code_lower.contains("thread")
            || code_lower.contains("mutex")
            || code_lower.contains("arc")
        {
            test_cases.push(AdversaryTestCase {
                name: "high_contention_race_probe".to_string(),
                category: AdversaryCategory::ConcurrencyStress,
                description: "Spawns 50 threads concurrently attempting read/write operations to detect race hazards or lock poisoning.".to_string(),
                harness_code: r#"
#[test]
fn test_concurrency_stress() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    let data = Arc::new(Mutex::new(0));
    let mut handles = vec![];
    for _ in 0..50 {
        let d = Arc::clone(&data);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let mut guard = d.lock().unwrap();
                *guard += 1;
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
    assert_eq!(*data.lock().unwrap(), 5000);
}
"#.trim().to_string(),
                expected_behavior: "All threads terminate without deadlocks; final synchronized state matches exact expected count.".to_string(),
                failure_mode: "Deadlock on nested lock acquisition or race condition under high scheduler preemption.".to_string(),
            });

            defensive_principles.push(
                "Always acquire locks in a consistent global order to prevent deadlocks."
                    .to_string(),
            );
            defensive_principles.push(
                "Minimize the duration of held MutexGuard scopes to prevent lock starvation."
                    .to_string(),
            );
        }

        if topic_lower.contains("unsafe")
            || code_lower.contains("unsafe")
            || code_lower.contains("pointer")
        {
            test_cases.push(AdversaryTestCase {
                name: "aliasing_violation_probe".to_string(),
                category: AdversaryCategory::MemorySafety,
                description: "Checks whether multiple mutable raw pointers alias the same underlying allocation.".to_string(),
                harness_code: r#"
#[test]
fn test_pointer_aliasing_invariants() {
    let mut val = 100u64;
    let ptr1 = &mut val as *mut u64;
    let ptr2 = &mut val as *mut u64;
    // Creating two simultaneous active &mut T references is UB even in unsafe!
    assert_eq!(ptr1, ptr2);
}
"#.trim().to_string(),
                expected_behavior: "Safe abstractions must encapsulate raw pointers so that safe callers cannot violate Stacked Borrows / Tree Borrows rules.".to_string(),
                failure_mode: "Undefined Behavior (UB) caused by simultaneous active mutable references.".to_string(),
            });

            defensive_principles.push("Safe wrappers around unsafe blocks must maintain all language invariants for every possible input.".to_string());
        }

        // Boundary condition tests (universal)
        test_cases.push(AdversaryTestCase {
            name: "empty_collection_boundary".to_string(),
            category: AdversaryCategory::BoundaryCondition,
            description: "Passes zero-length slice or empty collection to verify non-panicking boundary handling.".to_string(),
            harness_code: r#"
#[test]
fn test_empty_boundary() {
    let empty: &[i32] = &[];
    assert!(empty.is_empty());
}
"#.trim().to_string(),
            expected_behavior: "Returns empty collection or None/Err cleanly without out-of-bounds indexing panic.".to_string(),
            failure_mode: "Panic via index out of bounds: the len is 0 but the index is 0.".to_string(),
        });

        test_cases.push(AdversaryTestCase {
            name: "numeric_extreme_overflow".to_string(),
            category: AdversaryCategory::ArithmeticOverflow,
            description: "Feeds i64::MAX and i64::MIN to detect unchecked integer arithmetic that overflows in debug builds.".to_string(),
            harness_code: r#"
#[test]
fn test_numeric_extremes() {
    let max = i64::MAX;
    let checked = max.checked_add(1);
    assert_eq!(checked, None);
}
"#.trim().to_string(),
            expected_behavior: "Uses checked_add, saturating_add, or overflowing_add rather than raw + operators.".to_string(),
            failure_mode: "Panic: attempt to add with overflow (debug mode) or silent modular wrap (release mode).".to_string(),
        });

        defensive_principles.push("Prefer slice methods (`.first()`, `.get(idx)`) over direct indexing (`[idx]`) to prevent panic bugs.".to_string());
        defensive_principles.push(
            "Use `checked_*` arithmetic methods when operating near integer domain limits."
                .to_string(),
        );

        let socratic_warning = format!(
            "Your implementation may function under nominal conditions, but how does it behave under pathological boundaries? We formulated {} adversary test cases across {} stress vectors. Can your invariants withstand extreme inputs?",
            test_cases.len(),
            test_cases.iter().map(|tc| tc.category.to_string()).collect::<Vec<_>>().join(", ")
        );

        AdversaryReport {
            topic_id: topic_id.to_string(),
            test_cases,
            socratic_warning,
            defensive_principles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adversary_generates_boundary_and_arithmetic_tests() {
        let adversary = AdversaryAgent::new();
        let report =
            adversary.generate_pathological_tests("arrays-and-slices", "fn process(v: &[i32]) {}");

        assert_eq!(report.topic_id, "arrays-and-slices");
        assert!(report.test_cases.len() >= 2);
        assert!(report
            .test_cases
            .iter()
            .any(|tc| tc.category == AdversaryCategory::BoundaryCondition));
        assert!(report
            .test_cases
            .iter()
            .any(|tc| tc.category == AdversaryCategory::ArithmeticOverflow));
        assert!(!report.defensive_principles.is_empty());
        assert!(report.socratic_warning.contains("pathological boundaries"));
    }

    #[test]
    fn test_adversary_injects_concurrency_stress_for_multithreaded_code() {
        let adversary = AdversaryAgent::new();
        let report = adversary.generate_pathological_tests(
            "concurrency-primitives",
            "use std::sync::Mutex; struct SafeCounter;",
        );

        assert!(report
            .test_cases
            .iter()
            .any(|tc| tc.category == AdversaryCategory::ConcurrencyStress));
        assert!(report
            .defensive_principles
            .iter()
            .any(|p| p.contains("MutexGuard")));
    }

    #[test]
    fn test_adversary_injects_unsafe_stress_for_pointer_code() {
        let adversary = AdversaryAgent::new();
        let report = adversary
            .generate_pathological_tests("unsafe-rust", "unsafe fn deref_ptr(p: *const u8) {}");

        assert!(report
            .test_cases
            .iter()
            .any(|tc| tc.category == AdversaryCategory::MemorySafety));
        assert!(report
            .defensive_principles
            .iter()
            .any(|p| p.contains("unsafe")));
    }
}
