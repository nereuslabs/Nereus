//! The 4 specialized agents of the Nereus learning automaton.

pub mod diagnostic;
pub mod coach;
pub mod tutor;
pub mod examiner;

pub use coach::CoachAgent;
pub use diagnostic::DiagnosticAgent;
pub use examiner::ExaminerAgent;
pub use tutor::TutorAgent;
