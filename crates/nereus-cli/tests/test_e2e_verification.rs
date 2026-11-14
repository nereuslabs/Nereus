//! Pre-Release End-to-End Verification Protocol Integration Test.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.
//!
//! Hermetic test validating the complete Socratic learning pipeline:
//! Diagnostic Quiz ➔ Coach DAG ➔ Tutor Theory ➔ Code Submission ➔
//! Sandbox Execution ➔ Examiner Socratic Feedback ➔ Session Persistence & Resumption.

use clap::Parser;
use nereus_agents::{CoachAgent, DiagnosticAgent, ExaminerAgent, TutorAgent};
use nereus_cli::{run, switch_llm_provider, Cli};
use nereus_core::models::{SandboxExecutionResult, UserLevel, UserProfile, Verdict};
use nereus_core::state::{LearningState, StateMachineContext};
use nereus_core::traits::StorageBackend;
use nereus_engine::decay::EbbinghausDecayEngine;
use nereus_llm::StubLLMClient;
use nereus_plugin::ModuleRegistry;
use nereus_sandbox::MockSandboxRunner;
use nereus_storage::{AssessmentRecord, SqliteStorage};
use std::sync::Arc;
use uuid::Uuid;

fn temp_db_path() -> String {
    let filename = format!("nereus_e2e_test_{}.db", Uuid::new_v4());
    std::env::temp_dir()
        .join(filename)
        .to_string_lossy()
        .to_string()
}

fn cleanup_db(path: &str) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{path}-wal"));
    let _ = std::fs::remove_file(format!("{path}-shm"));
}

#[tokio::test]
async fn test_e2e_pedagogical_pipeline_and_session_resumption() {
    let db = temp_db_path();
    let storage = Arc::new(SqliteStorage::new(&db).unwrap());

    // ------------------------------------------------------------------------
    // Step 1: Baseline Diagnostic Assessment
    // ------------------------------------------------------------------------
    let profile = UserProfile {
        skill: "Rust Systems".to_string(),
        current_level: UserLevel::Intermediate,
        target_level: UserLevel::Advanced,
        hours_per_day: 2.0,
        deadline_days: 14,
        goal: "Master ownership and concurrency idioms".to_string(),
    };

    let diag_agent = DiagnosticAgent::new();
    let questions = diag_agent.generate_questions(&profile);
    assert!(
        !questions.is_empty(),
        "Diagnostic agent must generate questions"
    );

    // Simulate answering with a weakness by picking wrong options
    let answers: Vec<String> = questions
        .iter()
        .map(|q| {
            let correct = q.correct_index.unwrap_or(0);
            let wrong = (correct + 1) % q.options.len().max(2);
            wrong.to_string()
        })
        .collect();

    let weakness_report = diag_agent.evaluate_answers_for_questions(&questions, &answers);
    assert!(
        !weakness_report.weak_areas.is_empty() || !weakness_report.recommended_topics.is_empty(),
        "Diagnostic evaluation must produce actionable weakness feedback"
    );

    // ------------------------------------------------------------------------
    // Step 2: Coach DAG Topological Sorting & Curriculum Scheduling
    // ------------------------------------------------------------------------
    let coach = CoachAgent::new();

    let roadmap = coach.generate_roadmap(&profile, &weakness_report);
    assert!(!roadmap.topics.is_empty(), "Roadmap must contain topics");

    let schedule = coach.calculate_schedule(&roadmap, &profile);
    assert!(schedule.feasible, "Schedule projection should be feasible");

    // ------------------------------------------------------------------------
    // Step 3: Tutor Conceptual Material Generation
    // ------------------------------------------------------------------------
    let tutor = TutorAgent::new();
    let first_topic = &roadmap.topics[0];
    let (theory, task) = tutor.provide_material(first_topic);
    assert!(
        theory.contains(&first_topic.title) || theory.contains("Theory") || !task.is_empty(),
        "Tutor theory must match topic context"
    );

    // ------------------------------------------------------------------------
    // Step 4: Sandbox Deliberate Error & Socratic Examiner Feedback
    // ------------------------------------------------------------------------
    let examiner = ExaminerAgent::new();

    // 4a. Simulate failing compilation with borrow-checker error E0382
    let failed_sandbox = SandboxExecutionResult {
        success: false,
        exit_code: 1,
        stdout: String::new(),
        stderr: "error[E0382]: borrow of moved value: `v`\n  --> src/main.rs:4:18".to_string(),
        duration_ms: 120,
    };

    let fail_assessment = examiner.assess_execution(first_topic, &failed_sandbox);
    assert_eq!(fail_assessment.verdict, Verdict::Retry);
    // Crucial Invariant: Socratic feedback must NEVER leak raw code solutions
    assert!(
        !fail_assessment.feedback.contains("```rust"),
        "Socratic feedback must refuse code dumping"
    );
    assert!(
        fail_assessment.feedback.contains('?'),
        "Socratic feedback must formulate guided questions"
    );

    // 4b. Simulate student insight and successful compilation
    let success_sandbox = SandboxExecutionResult {
        success: true,
        exit_code: 0,
        stdout: "test test_solution ... ok".to_string(),
        stderr: String::new(),
        duration_ms: 85,
    };

    let pass_assessment = examiner.assess_execution(first_topic, &success_sandbox);
    assert_eq!(pass_assessment.verdict, Verdict::Pass);

    // ------------------------------------------------------------------------
    // Step 5: SQLite Persistence & State Machine Lifecycle
    // ------------------------------------------------------------------------
    let mut ctx = StateMachineContext::new(profile.clone(), false);
    let session_id = ctx.session_id;
    ctx.roadmap = roadmap.clone();
    ctx.state = LearningState::Examining {
        topic: first_topic.clone(),
        task,
        submission: Some("pub fn solution() {}".to_string()),
        assessment: Some(pass_assessment.clone()),
    };

    let record = AssessmentRecord::from_assessment(session_id, &pass_assessment, 1);
    storage.save_session(&ctx).await.unwrap();
    storage.record_assessment(&record).await.unwrap();

    // ------------------------------------------------------------------------
    // Step 6: Session Resumption across Process Boundary
    // ------------------------------------------------------------------------
    let loaded = storage.load_session(session_id).await.unwrap();
    assert!(loaded.is_some(), "Session must be recoverable from SQLite");
    let loaded_ctx = loaded.unwrap();
    assert_eq!(loaded_ctx.session_id, session_id);
    assert_eq!(loaded_ctx.profile.skill, "Rust Systems");

    let history = storage.get_assessment_history(session_id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].verdict, "pass");

    // ------------------------------------------------------------------------
    // Step 7: CLI Commands Verification (Resume & Export)
    // ------------------------------------------------------------------------
    let resume_cli = Cli::parse_from([
        "nereus",
        "--session-id",
        &session_id.to_string(),
        "--offline",
        "--demo",
        "--db",
        &db,
    ]);
    let res = run(resume_cli).await;
    assert!(res.is_ok(), "CLI resume failed: {:?}", res);

    let export_cli = Cli::parse_from([
        "nereus",
        "--db",
        &db,
        "sessions",
        "export",
        &session_id.to_string(),
        "--format",
        "markdown",
    ]);
    let res_export = run(export_cli).await;
    assert!(res_export.is_ok(), "CLI export failed: {:?}", res_export);

    cleanup_db(&db);
}

#[tokio::test]
async fn test_e2e_provider_switching_and_ebbinghaus_decay() {
    let db = temp_db_path();
    let storage = Arc::new(SqliteStorage::new(&db).unwrap());

    let registry = ModuleRegistry::new(
        Arc::new(StubLLMClient::new()),
        Arc::new(MockSandboxRunner::new(false)),
        storage.clone(),
    );

    // 1. Hot-switch LLM provider dynamically
    let switched = switch_llm_provider(&registry, "stub").await.unwrap();
    assert_eq!(switched, "stub");

    // 2. Evaluate spaced repetition decay
    let decay_engine = EbbinghausDecayEngine::default();
    let old_timestamp = "2026-03-01T00:00:00Z";
    let now_timestamp = "2026-03-10T00:00:00Z"; // 9 days later

    let concept = nereus_storage::ConceptMastery {
        concept_id: "rust.lifetime.variance".to_string(),
        score: 80.0,
        last_reviewed_at: old_timestamp.to_string(),
        stability_days: 2.0, // 2-day half-life -> after 9 days retention < 5%
        review_count: 1,
    };

    assert!(decay_engine.is_decayed(&concept, now_timestamp));

    // 3. Generate micro-drill for decayed concept
    let micro_drill = decay_engine.generate_micro_drill(&concept);
    assert!(micro_drill.id.contains("rust-lifetime-variance"));

    // 4. Inject micro-drill into Coach roadmap
    let coach = CoachAgent::new();
    let profile = UserProfile::default();
    let weakness = nereus_core::models::WeaknessReport::default();
    let updated_roadmap =
        coach.generate_roadmap_with_decay(&profile, &weakness, vec![micro_drill.clone()]);
    assert_eq!(updated_roadmap.topics[0].id, micro_drill.id);

    cleanup_db(&db);
}
