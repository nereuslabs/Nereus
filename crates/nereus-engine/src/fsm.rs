//! Algebraic Finite State Machine orchestrator.

use crate::evaluator::{compute_pass_threshold, evaluate_verdict, sanitize_socratic_feedback};
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::events::DomainEvent;
use nereus_core::models::{Assessment, DiagnosticQuestion, Roadmap, RoadmapTopic, UserProfile, Verdict, WeaknessReport};
use nereus_core::state::{LearningState, StateMachineContext};
use nereus_plugin::ModuleRegistry;
use tracing::{debug, info, warn};

/// The central automaton driving Socratic learning state transitions.
pub struct LearningAutomaton {
    pub registry: ModuleRegistry,
    pub ctx: StateMachineContext,
}

impl LearningAutomaton {
    pub fn new(registry: ModuleRegistry, profile: UserProfile, run_diagnostic: bool) -> Self {
        let ctx = StateMachineContext::new(profile, run_diagnostic);
        Self { registry, ctx }
    }

    pub fn from_context(registry: ModuleRegistry, ctx: StateMachineContext) -> Self {
        Self { registry, ctx }
    }

    /// Advance the state machine by executing the current state's transition logic.
    pub async fn step(&mut self) -> CoreResult<LearningState> {
        info!("Stepping automaton in state: {}", self.ctx.state.name());

        match &self.ctx.state {
            LearningState::Diagnostic { questions, current_index, answers } => {
                let questions = questions.clone();
                let current_index = *current_index;
                let answers = answers.clone();

                if questions.is_empty() {
                    // Seed initial diagnostic questions
                    let initial_questions = vec![
                        DiagnosticQuestion {
                            id: "diag-1".to_string(),
                            question: format!(
                                "In {}, how do you guarantee data race freedom across threads?",
                                self.ctx.profile.skill
                            ),
                            options: vec![
                                "Global Interpreter Lock (GIL)".to_string(),
                                "Send/Sync type markers and ownership semantics".to_string(),
                                "Manual pointer checks at runtime".to_string(),
                            ],
                            correct_index: Some(1),
                        },
                        DiagnosticQuestion {
                            id: "diag-2".to_string(),
                            question: "What is the difference between Arc and Rc in Rust?".to_string(),
                            options: vec![
                                "Arc uses atomic operations for reference counting; Rc is non-atomic".to_string(),
                                "Rc can be sent across threads safely; Arc cannot".to_string(),
                                "Arc is faster than Rc for single-threaded tasks".to_string(),
                            ],
                            correct_index: Some(0),
                        },
                    ];

                    self.ctx.state = LearningState::Diagnostic {
                        questions: initial_questions,
                        current_index: 0,
                        answers: vec![],
                    };
                } else if current_index >= questions.len() {
                    // All diagnostic questions answered -> synthesize WeaknessReport and move to Coaching
                    let weakness_report = WeaknessReport {
                        weak_areas: vec!["Concurrency".to_string(), "Memory Management".to_string()],
                        recommended_topics: vec!["Arc & Mutex".to_string(), "Async Channels".to_string()],
                    };

                    info!("Diagnostic completed with weak areas: {:?}", weakness_report.weak_areas);
                    self.ctx.state = LearningState::Coaching { weakness_report };
                } else {
                    // Still in progress waiting for answers
                    debug!("Awaiting diagnostic answer for question {}", current_index);
                }
            }

            LearningState::Coaching { weakness_report } => {
                let topics = vec![
                    RoadmapTopic {
                        id: "topic-1".to_string(),
                        title: "Memory Safety & Ownership Semantics".to_string(),
                        description: "Deep dive into move semantics, borrowing rules, and RAII.".to_string(),
                        difficulty: 0.3,
                        prerequisites: vec![],
                        estimated_hours: 2.0,
                    },
                    RoadmapTopic {
                        id: "topic-2".to_string(),
                        title: "Concurrency Primitives & Tokio MPSC".to_string(),
                        description: "Mastering Arc, Mutex, async tasks, and thread synchronization.".to_string(),
                        difficulty: 0.7,
                        prerequisites: vec!["topic-1".to_string()],
                        estimated_hours: 4.0,
                    },
                ];

                self.ctx.roadmap = Roadmap { topics };
                self.ctx.current_topic_index = 0;

                if let Some(first_topic) = self.ctx.current_topic().cloned() {
                    let material = format!("# {}\n\n{}", first_topic.title, first_topic.description);
                    let task = format!("Implement a working challenge demonstrating: {}", first_topic.title);

                    self.ctx.state = LearningState::Tutoring {
                        topic: first_topic,
                        material,
                        task,
                        retry_count: 0,
                    };
                } else {
                    self.ctx.state = LearningState::Completed;
                }
            }

            LearningState::Tutoring { topic, material: _, task, retry_count } => {
                // Transition to Examining, ready for student submission
                self.ctx.state = LearningState::Examining {
                    topic: topic.clone(),
                    task: task.clone(),
                    submission: None,
                    assessment: None,
                };
            }

            LearningState::Examining { topic, task: _, submission, assessment: _ } => {
                let topic = topic.clone();
                let sub_str = submission.clone().unwrap_or_default();

                // 1. Run student code through sandbox
                let sandbox = self.registry.get_sandbox().await;
                let sandbox_result = sandbox.execute(&topic, &sub_str).await?;

                // 2. Score assessment
                let score = if sandbox_result.success { 90.0 } else { 55.0 };
                let verdict = evaluate_verdict(&topic, score);
                let raw_feedback = if verdict == Verdict::Pass {
                    "Sandbox assertions verified successfully."
                } else {
                    "Notice compiler error E0382: ownership moved. How does cloning the sender resolve the borrow?"
                };

                let feedback = sanitize_socratic_feedback(raw_feedback, verdict);

                let assessment = Assessment {
                    topic_id: topic.id.clone(),
                    score,
                    verdict,
                    feedback,
                    weak_areas: if verdict == Verdict::Pass { vec![] } else { vec!["Ownership".to_string()] },
                };

                if verdict == Verdict::Pass {
                    info!("Topic {} passed with score {}", topic.title, score);
                    self.ctx.current_topic_index += 1;

                    if let Some(next_topic) = self.ctx.current_topic().cloned() {
                        let material = format!("# {}\n\n{}", next_topic.title, next_topic.description);
                        let task = format!("Solve the challenge for: {}", next_topic.title);
                        self.ctx.state = LearningState::Tutoring {
                            topic: next_topic,
                            material,
                            task,
                            retry_count: 0,
                        };
                    } else {
                        self.ctx.state = LearningState::Completed;
                    }
                } else {
                    warn!("Topic {} failed with score {}. Triggering Socratic retry.", topic.title, score);
                    let material = format!("# Revision: {}\n\nSocratic Hint: {}", topic.title, assessment.feedback);
                    let task = format!("Revise your solution for: {}", topic.title);
                    self.ctx.state = LearningState::Tutoring {
                        topic,
                        material,
                        task,
                        retry_count: 1,
                    };
                }
            }

            LearningState::Completed => {
                info!("Automaton has reached terminal state: Completed");
            }
        }

        // Automatically persist state if storage backend is wired
        let storage = self.registry.get_storage().await;
        let _ = storage.save_session(&self.ctx).await;

        Ok(self.ctx.state.clone())
    }
}
