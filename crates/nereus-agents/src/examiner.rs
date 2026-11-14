//! Examiner Agent: Assesses sandbox execution output and delivers Socratic guidance.
//!
//! Calculates dynamic passing thresholds using `(70.0 + topic.difficulty * 15.0).min(85.0)`
//! and enforces Socratic pedagogy: never reveals code, but provides conceptual questions
//! and diagnostic pointers to guide the student toward independent insight.

use async_trait::async_trait;
use nereus_core::models::{Assessment, RoadmapTopic, SandboxExecutionResult, Verdict};
use nereus_core::traits::Agent;
use serde::{Deserialize, Serialize};

/// Socratic guidance evaluation and compliance verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocraticGuidance {
    pub diagnostic_pointer: String,
    pub conceptual_question: String,
    pub identified_weak_areas: Vec<String>,
}

/// Examiner Agent enforcing rigorous verification and Socratic feedback rules.
#[derive(Debug, Default, Clone)]
pub struct ExaminerAgent;

impl ExaminerAgent {
    pub fn new() -> Self {
        Self
    }

    /// Compute dynamic passing threshold based on topic difficulty:
    /// Formula: `(70.0 + difficulty * 15.0).min(85.0)`
    pub fn compute_pass_threshold(difficulty: f32) -> f32 {
        (70.0 + difficulty * 15.0).min(85.0)
    }

    /// Evaluates raw numerical score against difficulty-scaled threshold to determine verdict.
    pub fn evaluate_verdict(&self, topic: &RoadmapTopic, score: f32) -> Verdict {
        let threshold = Self::compute_pass_threshold(topic.difficulty);
        if score >= threshold {
            Verdict::Pass
        } else {
            Verdict::Retry
        }
    }

    /// Analyzes sandbox compilation/execution output and produces an official Socratic Assessment.
    pub fn assess_execution(
        &self,
        topic: &RoadmapTopic,
        sandbox: &SandboxExecutionResult,
    ) -> Assessment {
        let threshold = Self::compute_pass_threshold(topic.difficulty);

        if sandbox.success {
            let score = 95.0_f32;
            Assessment {
                topic_id: topic.id.clone(),
                score,
                verdict: Verdict::Pass,
                feedback: format!(
                    "Mastery verified for «{}». All compiler checks and test assertions passed (score: {:.1}, threshold: {:.1}).",
                    topic.title, score, threshold
                ),
                weak_areas: vec![],
            }
        } else {
            let guidance = self.generate_socratic_guidance(topic, &sandbox.stderr, &sandbox.stdout);

            // Compute score based on failure characteristics
            let score = if sandbox.stderr.contains("error[E") {
                50.0 // Compiler error
            } else if sandbox.stderr.contains("assertion failed")
                || sandbox.stderr.contains("panicked")
            {
                60.0 // Runtime logic error
            } else if sandbox.exit_code == 124 || sandbox.duration_ms > 10000 {
                40.0 // Timeout or hang
            } else {
                55.0 // General failure
            };

            let combined_feedback = format!(
                "Diagnostic Pointer: {}\nReflective Question: {}",
                guidance.diagnostic_pointer, guidance.conceptual_question
            );

            // Ensure Socratic compliance (never leaks code)
            let sanitized_feedback =
                Self::sanitize_socratic_feedback(&combined_feedback, Verdict::Retry);

            Assessment {
                topic_id: topic.id.clone(),
                score,
                verdict: Verdict::Retry,
                feedback: sanitized_feedback,
                weak_areas: guidance.identified_weak_areas,
            }
        }
    }

    /// Directly assess a numerical score against a topic with Socratic feedback sanitization.
    pub fn assess_score(&self, topic: &RoadmapTopic, score: f32, raw_feedback: &str) -> Assessment {
        let verdict = self.evaluate_verdict(topic, score);
        let feedback = Self::sanitize_socratic_feedback(raw_feedback, verdict);

        let weak_areas = if verdict == Verdict::Retry {
            vec![topic.title.clone()]
        } else {
            vec![]
        };

        Assessment {
            topic_id: topic.id.clone(),
            score,
            verdict,
            feedback,
            weak_areas,
        }
    }

    /// Extract diagnostic pointers and Socratic reflective questions without revealing code solutions.
    pub fn generate_socratic_guidance(
        &self,
        topic: &RoadmapTopic,
        stderr: &str,
        stdout: &str,
    ) -> SocraticGuidance {
        let combined = format!("{stderr}\n{stdout}");

        if combined.contains("E0382") {
            SocraticGuidance {
                diagnostic_pointer: "Compiler diagnostic E0382 detected: a value was accessed after its ownership had already been moved.".to_string(),
                conceptual_question: "Trace where the value was passed by value. Can the operation proceed using a shared borrow (&T) or an atomic reference counted wrapper (Arc) instead of relinquishing ownership?".to_string(),
                identified_weak_areas: vec!["Ownership & Move Semantics (E0382)".to_string()],
            }
        } else if combined.contains("E0597") {
            SocraticGuidance {
                diagnostic_pointer: "Compiler diagnostic E0597 detected: a borrowed value does not live long enough to satisfy reference requirements.".to_string(),
                conceptual_question: "Compare the scope of the underlying owner against the lifetime of the reference. How does storing or returning a reference require the owner to outlive the receiving structure?".to_string(),
                identified_weak_areas: vec!["Lifetime Scopes & Drop Order (E0597)".to_string()],
            }
        } else if combined.contains("E0502") {
            SocraticGuidance {
                diagnostic_pointer: "Compiler diagnostic E0502 detected: cannot borrow as mutable because the value is concurrently borrowed as immutable.".to_string(),
                conceptual_question: "How does the Aliasing XOR Mutability principle protect against iterator invalidation and data races? Can you separate the read phase from the subsequent mutation phase?".to_string(),
                identified_weak_areas: vec!["Borrow Checker Aliasing Invariant (E0502)".to_string()],
            }
        } else if combined.contains("E0277") {
            SocraticGuidance {
                diagnostic_pointer: "Compiler diagnostic E0277 detected: a required trait bound (such as Send, Sync, or Clone) is not satisfied.".to_string(),
                conceptual_question: "Which type in your struct or closure fails to satisfy the required trait contract? For cross-thread concurrency, why does the compiler reject non-atomic types like Rc or RefCell?".to_string(),
                identified_weak_areas: vec!["Trait Bounds & Concurrency Markers (E0277)".to_string()],
            }
        } else if combined.contains("E0499") {
            SocraticGuidance {
                diagnostic_pointer: "Compiler diagnostic E0499 detected: cannot borrow as mutable more than once at a time in this scope.".to_string(),
                conceptual_question: "Where does the first exclusive borrow begin, and where does its usage conclude? Can the operations be sequenced into distinct lexical scopes or split across disjoint struct fields?".to_string(),
                identified_weak_areas: vec!["Exclusive Borrowing Scoping (E0499)".to_string()],
            }
        } else if combined.contains("assertion failed") || combined.contains("panicked") {
            SocraticGuidance {
                diagnostic_pointer: "Test assertion or panic triggered during sandbox verification.".to_string(),
                conceptual_question: "Examine the boundary conditions of your algorithm. What assumptions did the test violate (e.g., empty collection, zero value, or concurrency race)?".to_string(),
                identified_weak_areas: vec!["Algorithm Invariants & Boundary Handling".to_string()],
            }
        } else {
            SocraticGuidance {
                diagnostic_pointer: format!("Verification did not pass for «{}».", topic.title),
                conceptual_question: "Review the topic description and required type signatures. What invariant is your implementation currently violating?".to_string(),
                identified_weak_areas: vec![topic.title.clone()],
            }
        }
    }

    /// Enforces Socratic guidance rules:
    /// Strips any raw solution code, ensures feedback prompts thinking, and wraps with pedagogical framing.
    pub fn sanitize_socratic_feedback(raw_feedback: &str, verdict: Verdict) -> String {
        match verdict {
            Verdict::Pass => {
                if raw_feedback.is_empty() {
                    "Mastery confirmed. Invariants satisfied.".to_string()
                } else {
                    raw_feedback.to_string()
                }
            }
            Verdict::Retry => {
                // Strip code blocks if inadvertently present
                let mut sanitized = raw_feedback.to_string();
                if sanitized.contains("```") {
                    sanitized = sanitized
                        .lines()
                        .filter(|line| !line.trim_start().starts_with("```"))
                        .collect::<Vec<_>>()
                        .join("\n");
                }

                // If feedback lacks a guiding question, append a Socratic prompt
                if !sanitized.contains('?') {
                    sanitized.push_str("\nReflective Question: What core invariant was violated by this execution?");
                }

                sanitized
            }
        }
    }

    /// Verifies whether a piece of feedback strictly complies with Socratic rules.
    ///
    /// Socratic rules:
    /// 1. Must NOT contain code blocks (```).
    /// 2. Must NOT contain explicit implementation keywords (`fn `, `let mut `, `impl `).
    /// 3. For retry feedback, must contain a reflective question mark (`?`).
    pub fn is_socratic_compliant(feedback: &str, verdict: Verdict) -> bool {
        if feedback.contains("```") {
            return false;
        }

        // Check for leaked implementation code
        let leaked_patterns = ["fn main", "fn ", "let mut ", "impl ", "pub struct "];
        for pattern in &leaked_patterns {
            if feedback.contains(pattern) {
                return false;
            }
        }

        !(verdict == Verdict::Retry && !feedback.contains('?'))
    }
}

#[async_trait]
impl Agent for ExaminerAgent {
    fn id(&self) -> &'static str {
        "examiner-agent"
    }

    fn name(&self) -> &'static str {
        "Examiner Agent"
    }
}
