//! Nereus Agents: Diagnostic, Coach, Tutor, and Examiner implementations.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod coach;
pub mod diagnostic;
pub mod examiner;
pub mod tutor;

pub use coach::CoachAgent;
pub use diagnostic::DiagnosticAgent;
pub use examiner::ExaminerAgent;
pub use tutor::TutorAgent;
