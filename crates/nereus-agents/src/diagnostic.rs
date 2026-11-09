//! Diagnostic Agent: Evaluates baseline skills before roadmap generation.
//!
//! Provides a multi-category technical question bank covering core Rust systems programming
//! topics and computes structured weakness reports with severity levels.

use async_trait::async_trait;
use nereus_core::models::{DiagnosticQuestion, UserLevel, UserProfile, WeaknessReport};
use nereus_core::traits::Agent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Technical topic categories evaluated during diagnostic probing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuestionCategory {
    Ownership,
    Borrowing,
    Lifetimes,
    TokioConcurrency,
    ErrorHandling,
    TraitBounds,
}

impl QuestionCategory {
    /// Human-readable display label for the category.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ownership => "Ownership",
            Self::Borrowing => "Borrowing",
            Self::Lifetimes => "Lifetimes",
            Self::TokioConcurrency => "Tokio Concurrency",
            Self::ErrorHandling => "Error Handling",
            Self::TraitBounds => "Trait Bounds",
        }
    }

    /// Default curriculum topics recommended to remediate weaknesses in this category.
    pub fn default_recommended_topics(&self) -> Vec<String> {
        match self {
            Self::Ownership => vec![
                "Memory Safety & RAII Semantics".to_string(),
                "Move Semantics & Copy vs Clone".to_string(),
            ],
            Self::Borrowing => vec![
                "Borrow Checker & Exclusive Mutability".to_string(),
                "Non-Lexical Lifetimes & Reborrowing".to_string(),
            ],
            Self::Lifetimes => vec![
                "Lifetime Annotations & Elision Rules".to_string(),
                "Structs with References & 'static Bounds".to_string(),
            ],
            Self::TokioConcurrency => vec![
                "Tokio Runtime & Cooperative Scheduling".to_string(),
                "Async Channels & Mutex Synchronization".to_string(),
            ],
            Self::ErrorHandling => vec![
                "Result, Option & Try Operator (?)".to_string(),
                "Custom Error Hierarchies with thiserror".to_string(),
            ],
            Self::TraitBounds => vec![
                "Trait Bounds & Static Dispatch".to_string(),
                "Dynamic Dispatch (dyn Trait) & Object Safety".to_string(),
            ],
        }
    }
}

/// Severity classification for identified knowledge gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl SeverityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    /// Derive severity from category accuracy (0.0 to 1.0).
    pub fn from_accuracy(accuracy: f32) -> Option<Self> {
        if accuracy >= 1.0 {
            None
        } else if accuracy < 0.25 {
            Some(Self::Critical)
        } else if accuracy < 0.50 {
            Some(Self::High)
        } else if accuracy < 0.75 {
            Some(Self::Medium)
        } else {
            Some(Self::Low)
        }
    }
}

/// An entry in the diagnostic technical question bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionBankItem {
    pub id: String,
    pub category: QuestionCategory,
    pub difficulty: f32,
    pub question: DiagnosticQuestion,
    pub explanation: String,
}

/// Granular score summary for an individual technical category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryScore {
    pub category: QuestionCategory,
    pub total: usize,
    pub correct: usize,
    pub accuracy: f32,
    pub severity: Option<SeverityLevel>,
}

/// Structured weakness description pairing category, severity, and analytical context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizedWeakness {
    pub category: QuestionCategory,
    pub severity: SeverityLevel,
    pub description: String,
}

/// Detailed evaluation report with breakdown across categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedWeaknessReport {
    pub category_scores: Vec<CategoryScore>,
    pub overall_score: f32,
    pub weaknesses: Vec<CategorizedWeakness>,
    pub recommended_topics: Vec<String>,
}

impl DetailedWeaknessReport {
    /// Converts detailed report into standard `nereus_core::models::WeaknessReport`.
    pub fn to_core_report(&self) -> WeaknessReport {
        let weak_areas = self
            .weaknesses
            .iter()
            .map(|w| format!("{} [{}]", w.category.as_str(), w.severity.as_str()))
            .collect();

        WeaknessReport {
            weak_areas,
            recommended_topics: self.recommended_topics.clone(),
        }
    }
}

/// Diagnostic Agent evaluating baseline competency through structured questioning.
#[derive(Debug, Clone)]
pub struct DiagnosticAgent {
    bank: Vec<QuestionBankItem>,
}

impl Default for DiagnosticAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticAgent {
    /// Initialize a DiagnosticAgent populated with the multi-category technical question bank.
    pub fn new() -> Self {
        let bank = vec![
            // 1. Ownership
            QuestionBankItem {
                id: "diag-own-1".to_string(),
                category: QuestionCategory::Ownership,
                difficulty: 0.2,
                question: DiagnosticQuestion {
                    id: "diag-own-1".to_string(),
                    question: "In Rust, what occurs when a non-Copy type is assigned to a new variable or passed by value?".to_string(),
                    options: vec![
                        "It is bitwise duplicated and both bindings remain independently valid".to_string(),
                        "Ownership moves to the new binding; the original variable becomes uninitialized and invalid".to_string(),
                        "An atomic reference counter is automatically allocated on the heap".to_string(),
                        "A runtime panic occurs if accessed again without an explicit clone".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "Rust's affine type system enforces move semantics for non-Copy types by default, invalidating the previous binding at compile time.".to_string(),
            },
            QuestionBankItem {
                id: "diag-own-2".to_string(),
                category: QuestionCategory::Ownership,
                difficulty: 0.5,
                question: DiagnosticQuestion {
                    id: "diag-own-2".to_string(),
                    question: "What is the primary guarantee provided by the Drop trait in Rust?".to_string(),
                    options: vec![
                        "Marks types that cannot be sent across thread boundaries".to_string(),
                        "Implements RAII cleanup logic executed deterministically when an owner leaves scope".to_string(),
                        "Triggers garbage collection cycles to collect cyclic references".to_string(),
                        "Defers memory deallocation to the operating system at process exit".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "Drop specifies custom cleanup code that runs automatically when an owner goes out of scope, guaranteeing Resource Acquisition Is Initialization (RAII).".to_string(),
            },

            // 2. Borrowing
            QuestionBankItem {
                id: "diag-bor-1".to_string(),
                category: QuestionCategory::Borrowing,
                difficulty: 0.3,
                question: DiagnosticQuestion {
                    id: "diag-bor-1".to_string(),
                    question: "What core invariant does Rust's borrow checker enforce regarding references?".to_string(),
                    options: vec![
                        "Arbitrary numbers of mutable and immutable references can coexist if scoped".to_string(),
                        "Either any number of immutable references (&T) OR exactly one mutable reference (&mut T) at any time".to_string(),
                        "Mutable references must always target data on the heap".to_string(),
                        "References must be manually freed when exiting their lexical block".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "Aliasing XOR Mutability is Rust's primary safety invariant: concurrent readers are safe, or a single exclusive writer is safe, but never both.".to_string(),
            },
            QuestionBankItem {
                id: "diag-bor-2".to_string(),
                category: QuestionCategory::Borrowing,
                difficulty: 0.6,
                question: DiagnosticQuestion {
                    id: "diag-bor-2".to_string(),
                    question: "What capability does Non-Lexical Lifetimes (NLL) introduce to Rust's borrow checker?".to_string(),
                    options: vec![
                        "Allows references to outlive referenced data if annotated with 'static".to_string(),
                        "Ends a borrow at the point of its last actual usage rather than the end of the enclosing block".to_string(),
                        "Permits concurrent writes through raw pointers without unsafe blocks".to_string(),
                        "Automatically clones borrowed data when a conflicting mutable borrow occurs".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "NLL computes control-flow graph usage, terminating reference lifetimes after their last live read/write point.".to_string(),
            },

            // 3. Lifetimes
            QuestionBankItem {
                id: "diag-life-1".to_string(),
                category: QuestionCategory::Lifetimes,
                difficulty: 0.4,
                question: DiagnosticQuestion {
                    id: "diag-life-1".to_string(),
                    question: "Why do Rust functions returning references often require explicit generic lifetime parameters?".to_string(),
                    options: vec![
                        "To dynamically allocate stack frames for returned pointers".to_string(),
                        "To connect the returned reference's lifetime to the lifetime of input parameters at compile time".to_string(),
                        "To instruct the CPU to lock memory caches during function execution".to_string(),
                        "To override borrow checker invariants when passing references across crates".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "Lifetime annotations relate the output reference to one or more input parameters so the compiler ensures the caller does not use a dangling reference.".to_string(),
            },
            QuestionBankItem {
                id: "diag-life-2".to_string(),
                category: QuestionCategory::Lifetimes,
                difficulty: 0.7,
                question: DiagnosticQuestion {
                    id: "diag-life-2".to_string(),
                    question: "What does the trait bound `T: 'static` signify for a generic type T?".to_string(),
                    options: vec![
                        "T must be an immutable reference pointing to read-only program memory".to_string(),
                        "T can safely live for the entire program duration, meaning it contains no non-'static references".to_string(),
                        "T must be allocated exclusively on the heap via Box::leak".to_string(),
                        "T is forbidden from implementing the Drop trait".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "A `T: 'static` bound means T is either an owned type containing no references or contains only 'static references; it does not mean T must be a reference.".to_string(),
            },

            // 4. Tokio Concurrency
            QuestionBankItem {
                id: "diag-tok-1".to_string(),
                category: QuestionCategory::TokioConcurrency,
                difficulty: 0.5,
                question: DiagnosticQuestion {
                    id: "diag-tok-1".to_string(),
                    question: "Why must CPU-bound blocking operations be delegated to `tokio::task::spawn_blocking`?".to_string(),
                    options: vec![
                        "Async worker threads are running in WebAssembly sandbox containers".to_string(),
                        "Blocking worker threads starves the cooperative scheduler, preventing other concurrent tasks from progressing".to_string(),
                        "The compiler refuses to compile functions containing loops inside async fn".to_string(),
                        "Tokio tasks run on separate OS processes and cannot execute standard library code".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "Tokio utilizes cooperative multitasking on a small thread pool; blocking a worker thread stops all tasks scheduled on that thread.".to_string(),
            },
            QuestionBankItem {
                id: "diag-tok-2".to_string(),
                category: QuestionCategory::TokioConcurrency,
                difficulty: 0.7,
                question: DiagnosticQuestion {
                    id: "diag-tok-2".to_string(),
                    question: "What is the key semantic distinction between `tokio::sync::mpsc` and `tokio::sync::broadcast`?".to_string(),
                    options: vec![
                        "MPSC delivers each message to a single receiver; broadcast delivers every message to all active receivers".to_string(),
                        "Broadcast channels are strictly synchronous, while MPSC channels are asynchronous".to_string(),
                        "MPSC channels drop messages if buffer is full; broadcast blocks producers permanently".to_string(),
                        "MPSC cannot be shared across multiple threads, whereas broadcast can".to_string(),
                    ],
                    correct_index: Some(0),
                },
                explanation: "MPSC is a point-to-point multi-producer single-consumer queue, while broadcast is a multi-producer multi-consumer publish-subscribe channel.".to_string(),
            },

            // 5. Error Handling
            QuestionBankItem {
                id: "diag-err-1".to_string(),
                category: QuestionCategory::ErrorHandling,
                difficulty: 0.3,
                question: DiagnosticQuestion {
                    id: "diag-err-1".to_string(),
                    question: "How does the `?` try operator evaluate an `Err(e)` expression inside a function?".to_string(),
                    options: vec![
                        "It panics immediately and aborts the current thread".to_string(),
                        "It applies `From::from(e)` to convert the error and early-returns `Err` from the function".to_string(),
                        "It discards the error and substitutes the default value of the Ok variant".to_string(),
                        "It re-executes the preceding expression in a retry loop".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "The `?` operator performs early return of `Err(From::from(e))`, allowing clean error propagation and type conversion across domain boundaries.".to_string(),
            },
            QuestionBankItem {
                id: "diag-err-2".to_string(),
                category: QuestionCategory::ErrorHandling,
                difficulty: 0.6,
                question: DiagnosticQuestion {
                    id: "diag-err-2".to_string(),
                    question: "In idiomatic Rust libraries, why is `thiserror` preferred over `anyhow`?".to_string(),
                    options: vec![
                        "thiserror dynamically boxes errors on the heap to capture stack backtraces".to_string(),
                        "thiserror derives strongly-typed enum error definitions suitable for public APIs, while anyhow is designed for applications".to_string(),
                        "anyhow cannot be safely transmitted across thread boundaries".to_string(),
                        "thiserror eliminates the need to implement std::error::Error".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "Libraries should provide exhaustive, typed error enums via `thiserror` so callers can match on specific failure modes; `anyhow` is intended for application-level catch-all handling.".to_string(),
            },

            // 6. Trait bounds
            QuestionBankItem {
                id: "diag-tr-1".to_string(),
                category: QuestionCategory::TraitBounds,
                difficulty: 0.4,
                question: DiagnosticQuestion {
                    id: "diag-tr-1".to_string(),
                    question: "What is the primary architectural trade-off between generic static dispatch and `dyn Trait` dynamic dispatch?".to_string(),
                    options: vec![
                        "Static dispatch allocates on the heap; dynamic dispatch is zero-cost stack-allocated".to_string(),
                        "Static dispatch monomorphizes code for zero runtime cost; dynamic dispatch uses vtables, trading binary size for heterogeneous flexibility".to_string(),
                        "Dynamic dispatch checks method existence at runtime and can panic if missing".to_string(),
                        "Static dispatch cannot be used with traits having default method implementations".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "Generics generate specialized machine code per concrete type (monomorphization) with inlining, while `dyn Trait` uses fat pointers and virtual method tables.".to_string(),
            },
            QuestionBankItem {
                id: "diag-tr-2".to_string(),
                category: QuestionCategory::TraitBounds,
                difficulty: 0.7,
                question: DiagnosticQuestion {
                    id: "diag-tr-2".to_string(),
                    question: "How does the Rust compiler determine whether a type implements the `Send` and `Sync` auto traits?".to_string(),
                    options: vec![
                        "Every struct requires explicit `unsafe impl Send for T` and `unsafe impl Sync for T`".to_string(),
                        "A type is automatically `Send` if all its fields are `Send`, and `Sync` if `&T` is `Send`, preserving thread safety compositionally".to_string(),
                        "All types implementing `Clone` are automatically declared `Send` and `Sync`".to_string(),
                        "The compiler only checks Send and Sync when code is compiled in release mode".to_string(),
                    ],
                    correct_index: Some(1),
                },
                explanation: "`Send` and `Sync` are compiler auto-traits synthesized automatically based on the transitive properties of the type's constituent fields.".to_string(),
            },
        ];

        Self { bank }
    }

    /// Access the complete question bank.
    pub fn question_bank(&self) -> &[QuestionBankItem] {
        &self.bank
    }

    /// Filter questions belonging to a specific technical category.
    pub fn questions_by_category(&self, category: QuestionCategory) -> Vec<DiagnosticQuestion> {
        self.bank
            .iter()
            .filter(|item| item.category == category)
            .map(|item| item.question.clone())
            .collect()
    }

    /// Select diagnostic questions tailored to the learner's profile level.
    pub fn generate_questions(&self, profile: &UserProfile) -> Vec<DiagnosticQuestion> {
        let (min_diff, max_diff) = match profile.current_level {
            UserLevel::Beginner => (0.0, 0.45),
            UserLevel::Intermediate => (0.2, 0.75),
            UserLevel::Advanced => (0.4, 1.0),
        };

        // Ensure at least one question per category is selected
        let mut selected = Vec::new();
        for &category in &[
            QuestionCategory::Ownership,
            QuestionCategory::Borrowing,
            QuestionCategory::Lifetimes,
            QuestionCategory::TokioConcurrency,
            QuestionCategory::ErrorHandling,
            QuestionCategory::TraitBounds,
        ] {
            let cat_items: Vec<&QuestionBankItem> = self
                .bank
                .iter()
                .filter(|item| item.category == category)
                .collect();

            // Try to find one within target difficulty, or fall back to closest
            let preferred = cat_items
                .iter()
                .find(|item| item.difficulty >= min_diff && item.difficulty <= max_diff)
                .or_else(|| cat_items.first());

            if let Some(item) = preferred {
                selected.push(item.question.clone());
            }
        }

        selected
    }

    /// Parses a user's answer string and checks correctness against the target question.
    ///
    /// Supports:
    /// - 0-based index strings: "0", "1", "2", ...
    /// - Letter choices: "A", "B", "C", "D" (case-insensitive)
    /// - Full option text matching (case-insensitive trimmed)
    pub fn is_answer_correct(question: &DiagnosticQuestion, answer: &str) -> bool {
        let correct_idx = match question.correct_index {
            Some(idx) => idx,
            None => return false,
        };

        let trimmed = answer.trim();

        // 1. Try parsing as 0-based option index
        if let Ok(parsed_idx) = trimmed.parse::<usize>() {
            if parsed_idx == correct_idx {
                return true;
            }
        }

        // 2. Match letter choices A, B, C, D (case-insensitive)
        if trimmed.len() == 1 {
            let ch = trimmed.chars().next().unwrap().to_ascii_uppercase();
            if ch.is_ascii_uppercase() {
                let letter_idx = (ch as u8 - b'A') as usize;
                if letter_idx == correct_idx {
                    return true;
                }
            }
        }

        // 3. Match against option text
        if let Some(correct_text) = question.options.get(correct_idx) {
            if trimmed.eq_ignore_ascii_case(correct_text.trim()) {
                return true;
            }
        }

        false
    }

    /// Compute detailed diagnostic results across all categories for given questions and answers.
    pub fn evaluate_detailed(
        &self,
        questions: &[DiagnosticQuestion],
        answers: &[String],
    ) -> DetailedWeaknessReport {
        let mut category_stats: HashMap<QuestionCategory, (usize, usize)> = HashMap::new();

        // Map question ID to its bank item category
        let q_to_cat: HashMap<&str, QuestionCategory> = self
            .bank
            .iter()
            .map(|item| (item.id.as_str(), item.category))
            .collect();

        for (idx, q) in questions.iter().enumerate() {
            let cat = q_to_cat
                .get(q.id.as_str())
                .copied()
                .unwrap_or(QuestionCategory::Ownership);

            let entry = category_stats.entry(cat).or_insert((0, 0));
            entry.0 += 1; // total questions in category

            let ans = answers.get(idx).map(|s| s.as_str()).unwrap_or("");
            if Self::is_answer_correct(q, ans) {
                entry.1 += 1; // correct answers
            }
        }

        let mut category_scores = Vec::new();
        let mut weaknesses = Vec::new();
        let mut total_q = 0;
        let mut total_correct = 0;

        // Iterate categories deterministically
        for &category in &[
            QuestionCategory::Ownership,
            QuestionCategory::Borrowing,
            QuestionCategory::Lifetimes,
            QuestionCategory::TokioConcurrency,
            QuestionCategory::ErrorHandling,
            QuestionCategory::TraitBounds,
        ] {
            if let Some(&(total, correct)) = category_stats.get(&category) {
                total_q += total;
                total_correct += correct;
                let accuracy = if total > 0 {
                    correct as f32 / total as f32
                } else {
                    1.0
                };

                let severity = SeverityLevel::from_accuracy(accuracy);

                if let Some(sev) = severity {
                    let desc = format!(
                        "Accuracy {:.0}% ({}/{} correct) in {}",
                        accuracy * 100.0,
                        correct,
                        total,
                        category.as_str()
                    );
                    weaknesses.push(CategorizedWeakness {
                        category,
                        severity: sev,
                        description: desc,
                    });
                }

                category_scores.push(CategoryScore {
                    category,
                    total,
                    correct,
                    accuracy,
                    severity,
                });
            }
        }

        // Sort weaknesses by severity descending (Critical first)
        weaknesses.sort_by_key(|a| std::cmp::Reverse(a.severity));

        // Generate tailored recommendations prioritized by weakness severity
        let mut recommended_topics = Vec::new();
        for weakness in &weaknesses {
            for topic in weakness.category.default_recommended_topics() {
                if !recommended_topics.contains(&topic) {
                    recommended_topics.push(topic);
                }
            }
        }

        let overall_score = if total_q > 0 {
            (total_correct as f32 / total_q as f32) * 100.0
        } else {
            100.0
        };

        DetailedWeaknessReport {
            category_scores,
            overall_score,
            weaknesses,
            recommended_topics,
        }
    }

    /// Evaluates answers against specific questions and outputs standard `WeaknessReport`.
    pub fn evaluate_answers_for_questions(
        &self,
        questions: &[DiagnosticQuestion],
        answers: &[String],
    ) -> WeaknessReport {
        self.evaluate_detailed(questions, answers).to_core_report()
    }

    /// Evaluates user answers against default diagnostic questions.
    pub fn evaluate_answers(&self, answers: &[String]) -> WeaknessReport {
        let questions = self.generate_questions(&UserProfile::default());
        self.evaluate_answers_for_questions(&questions, answers)
    }
}

#[async_trait]
impl Agent for DiagnosticAgent {
    fn id(&self) -> &'static str {
        "diagnostic-agent"
    }

    fn name(&self) -> &'static str {
        "Diagnostic Agent"
    }
}
