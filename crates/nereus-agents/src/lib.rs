//! Nereus Agents: Diagnostic, Coach, Tutor, and Examiner implementations.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.
//!
//! Autonomous specialized agents implementing the Socratic learning loop:
//! - [`DiagnosticAgent`]: Evaluates baseline knowledge across core categories and identifies severity-weighted weaknesses.
//! - [`CoachAgent`]: Orders curriculum via DAG topological sorting, prioritizes weak areas, and computes completion schedules.
//! - [`TutorAgent`]: Generates high-density theory materials and dynamic compiler-diagnostic revisions.
//! - [`ExaminerAgent`]: Calculates difficulty-scaled passing thresholds and provides Socratic diagnostic guidance without revealing code.
//! - [`LibrarianAgent`]: Offline standard library and Nomicon documentation search with compiler error code citations.
//! - [`AdversaryAgent`]: Educational boundary-testing and property-based fuzzing agent injecting pathological test vectors.

pub mod adversary;
pub mod benchmark;
pub mod coach;
pub mod diagnostic;
pub mod examiner;
pub mod librarian;
pub mod tutor;

pub use adversary::{AdversaryAgent, AdversaryCategory, AdversaryReport, AdversaryTestCase};
pub use benchmark::{
    BenchmarkReport, BenchmarkTestCase, SocraticBenchmark, TestCaseResult,
    CANONICAL_BENCHMARK_CASES,
};
pub use coach::{CoachAgent, CoachError, MilestoneSchedule, RoadmapSchedule};
pub use diagnostic::{
    CategorizedWeakness, CategoryScore, DetailedWeaknessReport, DiagnosticAgent, QuestionBankItem,
    QuestionCategory, SeverityLevel,
};
pub use examiner::{ExaminerAgent, SocraticGuidance};
pub use librarian::{DocCategory, DocReference, LibrarianAgent, LibrarianResponse};
pub use tutor::{CompilerDiagnosticGuide, TutorAgent, COMPILER_GUIDES};
