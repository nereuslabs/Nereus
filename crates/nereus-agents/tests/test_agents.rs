//! Comprehensive unit and integration tests for Nereus specialized agents.
//! Verifies DiagnosticAgent, CoachAgent, TutorAgent, ExaminerAgent, and full pipeline flow.

use chrono::Utc;
use nereus_agents::{
    CoachAgent, CoachError, DiagnosticAgent, ExaminerAgent, QuestionCategory, SeverityLevel,
    TutorAgent, COMPILER_GUIDES,
};
use nereus_core::models::{
    Roadmap, RoadmapTopic, SandboxExecutionResult, UserLevel, UserProfile, Verdict, WeaknessReport,
};
use nereus_core::traits::Agent;

// ============================================================================
// 1. DiagnosticAgent Tests
// ============================================================================

#[test]
fn test_diagnostic_agent_identity() {
    let agent = DiagnosticAgent::new();
    assert_eq!(agent.id(), "diagnostic-agent");
    assert_eq!(agent.name(), "Diagnostic Agent");
}

#[test]
fn test_diagnostic_question_bank_covers_all_categories() {
    let agent = DiagnosticAgent::new();
    let bank = agent.question_bank();

    assert!(
        bank.len() >= 12,
        "Question bank must contain at least 12 questions"
    );

    let required_categories = [
        QuestionCategory::Ownership,
        QuestionCategory::Borrowing,
        QuestionCategory::Lifetimes,
        QuestionCategory::TokioConcurrency,
        QuestionCategory::ErrorHandling,
        QuestionCategory::TraitBounds,
    ];

    for cat in &required_categories {
        let items: Vec<_> = bank.iter().filter(|i| i.category == *cat).collect();
        assert!(
            items.len() >= 2,
            "Category {:?} must have at least 2 questions in bank, found {}",
            cat,
            items.len()
        );
        for item in items {
            assert!(
                item.question.correct_index.is_some(),
                "Question {} must have a correct_index",
                item.id
            );
            assert!(
                item.question.options.len() >= 3,
                "Question {} must have at least 3 options",
                item.id
            );
            assert!(
                !item.explanation.is_empty(),
                "Question {} must have an explanation",
                item.id
            );
        }
    }
}

#[test]
fn test_diagnostic_generate_questions_per_level() {
    let agent = DiagnosticAgent::new();

    let levels = [
        UserLevel::Beginner,
        UserLevel::Intermediate,
        UserLevel::Advanced,
    ];

    for level in levels {
        let profile = UserProfile {
            current_level: level,
            ..Default::default()
        };

        let questions = agent.generate_questions(&profile);
        assert_eq!(
            questions.len(),
            6,
            "Should generate one question per category for level {:?}",
            level
        );
    }
}

#[test]
fn test_diagnostic_answer_parsing_robustness() {
    let agent = DiagnosticAgent::new();
    let questions = agent.question_bank();
    let q = &questions[0].question;
    let correct_idx = q.correct_index.unwrap();

    // 0-based string index
    assert!(DiagnosticAgent::is_answer_correct(
        q,
        &correct_idx.to_string()
    ));

    // Letter choice (e.g., 'A' for 0, 'B' for 1)
    let letter = ((b'A' + correct_idx as u8) as char).to_string();
    assert!(DiagnosticAgent::is_answer_correct(q, &letter));
    assert!(DiagnosticAgent::is_answer_correct(
        q,
        &letter.to_lowercase()
    ));

    // Exact option text match
    assert!(DiagnosticAgent::is_answer_correct(
        q,
        &q.options[correct_idx]
    ));

    // Lowercase trimmed text match
    assert!(DiagnosticAgent::is_answer_correct(
        q,
        &q.options[correct_idx].to_lowercase()
    ));

    // Wrong answer
    let wrong_idx = (correct_idx + 1) % q.options.len();
    assert!(!DiagnosticAgent::is_answer_correct(
        q,
        &wrong_idx.to_string()
    ));
    assert!(!DiagnosticAgent::is_answer_correct(q, "invalid non-index"));
}

#[test]
fn test_diagnostic_scoring_perfect_score_produces_no_weaknesses() {
    let agent = DiagnosticAgent::new();
    let profile = UserProfile::default();
    let questions = agent.generate_questions(&profile);

    // Provide all correct answers
    let answers: Vec<String> = questions
        .iter()
        .map(|q| q.correct_index.unwrap().to_string())
        .collect();

    let detailed = agent.evaluate_detailed(&questions, &answers);
    assert_eq!(detailed.overall_score, 100.0);
    assert!(
        detailed.weaknesses.is_empty(),
        "Perfect score should yield 0 weaknesses"
    );
    assert!(detailed.recommended_topics.is_empty());

    let core_report = detailed.to_core_report();
    assert!(core_report.weak_areas.is_empty());
}

#[test]
fn test_diagnostic_scoring_critical_severity_on_complete_failure() {
    let agent = DiagnosticAgent::new();
    let profile = UserProfile::default();
    let questions = agent.generate_questions(&profile);

    // Provide all wrong answers
    let answers: Vec<String> = questions
        .iter()
        .map(|q| {
            let wrong = (q.correct_index.unwrap() + 1) % q.options.len();
            wrong.to_string()
        })
        .collect();

    let detailed = agent.evaluate_detailed(&questions, &answers);
    assert_eq!(detailed.overall_score, 0.0);
    assert_eq!(detailed.weaknesses.len(), 6);

    // All weaknesses should be Critical (0% accuracy < 25%)
    for w in &detailed.weaknesses {
        assert_eq!(w.severity, SeverityLevel::Critical);
    }

    let core_report = detailed.to_core_report();
    assert_eq!(core_report.weak_areas.len(), 6);
    assert!(core_report.weak_areas[0].contains("[Critical]"));
    assert!(!core_report.recommended_topics.is_empty());
}

#[test]
fn test_diagnostic_scoring_isolated_weakness_detection() {
    let agent = DiagnosticAgent::new();
    let profile = UserProfile::default();
    let questions = agent.generate_questions(&profile);

    // Answer correctly except for the question about Tokio Concurrency
    let q_to_cat: std::collections::HashMap<_, _> = agent
        .question_bank()
        .iter()
        .map(|i| (i.id.as_str(), i.category))
        .collect();

    let answers: Vec<String> = questions
        .iter()
        .map(|q| {
            let cat = q_to_cat.get(q.id.as_str()).unwrap();
            if *cat == QuestionCategory::TokioConcurrency {
                // Wrong answer
                let wrong = (q.correct_index.unwrap() + 1) % q.options.len();
                wrong.to_string()
            } else {
                // Correct answer
                q.correct_index.unwrap().to_string()
            }
        })
        .collect();

    let detailed = agent.evaluate_detailed(&questions, &answers);
    assert_eq!(detailed.weaknesses.len(), 1);
    assert_eq!(
        detailed.weaknesses[0].category,
        QuestionCategory::TokioConcurrency
    );
    assert_eq!(detailed.weaknesses[0].severity, SeverityLevel::Critical);

    let core_report = detailed.to_core_report();
    assert_eq!(core_report.weak_areas.len(), 1);
    assert!(core_report.weak_areas[0].starts_with("Tokio Concurrency [Critical]"));
    assert!(core_report
        .recommended_topics
        .iter()
        .any(|t| t.contains("Tokio")));
}

// ============================================================================
// 2. CoachAgent Tests
// ============================================================================

#[test]
fn test_coach_agent_identity() {
    let agent = CoachAgent::new();
    assert_eq!(agent.id(), "coach-agent");
    assert_eq!(agent.name(), "Coach Agent");
}

#[test]
fn test_coach_dag_topological_sort_linear() {
    let coach = CoachAgent::new();

    let topics = vec![
        RoadmapTopic {
            id: "t3".to_string(),
            title: "Topic 3".to_string(),
            description: "".to_string(),
            difficulty: 0.8,
            prerequisites: vec!["t2".to_string()],
            estimated_hours: 2.0,
        },
        RoadmapTopic {
            id: "t1".to_string(),
            title: "Topic 1".to_string(),
            description: "".to_string(),
            difficulty: 0.2,
            prerequisites: vec![],
            estimated_hours: 1.0,
        },
        RoadmapTopic {
            id: "t2".to_string(),
            title: "Topic 2".to_string(),
            description: "".to_string(),
            difficulty: 0.5,
            prerequisites: vec!["t1".to_string()],
            estimated_hours: 1.5,
        },
    ];

    let sorted = coach.topological_sort(&topics).expect("Should sort DAG");
    let sorted_ids: Vec<_> = sorted.iter().map(|t| t.id.as_str()).collect();

    assert_eq!(sorted_ids, vec!["t1", "t2", "t3"]);
}

#[test]
fn test_coach_dag_topological_sort_diamond() {
    let coach = CoachAgent::new();

    // Diamond: t1 -> t2, t1 -> t3, (t2, t3) -> t4
    let topics = vec![
        RoadmapTopic {
            id: "t4".to_string(),
            title: "Topic 4".to_string(),
            description: "".to_string(),
            difficulty: 0.9,
            prerequisites: vec!["t2".to_string(), "t3".to_string()],
            estimated_hours: 3.0,
        },
        RoadmapTopic {
            id: "t3".to_string(),
            title: "Topic 3".to_string(),
            description: "".to_string(),
            difficulty: 0.6,
            prerequisites: vec!["t1".to_string()],
            estimated_hours: 2.0,
        },
        RoadmapTopic {
            id: "t2".to_string(),
            title: "Topic 2".to_string(),
            description: "".to_string(),
            difficulty: 0.4,
            prerequisites: vec!["t1".to_string()],
            estimated_hours: 2.0,
        },
        RoadmapTopic {
            id: "t1".to_string(),
            title: "Topic 1".to_string(),
            description: "".to_string(),
            difficulty: 0.1,
            prerequisites: vec![],
            estimated_hours: 1.0,
        },
    ];

    let sorted = coach
        .topological_sort(&topics)
        .expect("Should sort diamond DAG");
    let sorted_ids: Vec<_> = sorted.iter().map(|t| t.id.as_str()).collect();

    // t1 must be first; t4 must be last; t2 and t3 must be in between
    assert_eq!(sorted_ids[0], "t1");
    assert_eq!(sorted_ids[3], "t4");
    assert!(sorted_ids[1] == "t2" || sorted_ids[1] == "t3");
}

#[test]
fn test_coach_dag_cycle_detection() {
    let coach = CoachAgent::new();

    // Cyclic: c1 -> c2 -> c3 -> c1
    let cyclic_topics = vec![
        RoadmapTopic {
            id: "c1".to_string(),
            title: "Cyclic 1".to_string(),
            description: "".to_string(),
            difficulty: 0.3,
            prerequisites: vec!["c3".to_string()],
            estimated_hours: 1.0,
        },
        RoadmapTopic {
            id: "c2".to_string(),
            title: "Cyclic 2".to_string(),
            description: "".to_string(),
            difficulty: 0.5,
            prerequisites: vec!["c1".to_string()],
            estimated_hours: 1.0,
        },
        RoadmapTopic {
            id: "c3".to_string(),
            title: "Cyclic 3".to_string(),
            description: "".to_string(),
            difficulty: 0.7,
            prerequisites: vec!["c2".to_string()],
            estimated_hours: 1.0,
        },
    ];

    let result = coach.topological_sort(&cyclic_topics);
    assert!(
        matches!(result, Err(CoachError::CycleDetected(_))),
        "Cycle must be detected"
    );
}

#[test]
fn test_coach_dynamic_reordering_prioritizes_weakness() {
    let coach = CoachAgent::new();

    // Two independent root topics: topic-ownership and topic-error (both have prereqs: [])
    // Normally topic-ownership comes first due to difficulty or id
    let topic_a = RoadmapTopic {
        id: "topic-a".to_string(),
        title: "Standard Ownership".to_string(),
        description: "Standard ownership semantics".to_string(),
        difficulty: 0.3,
        prerequisites: vec![],
        estimated_hours: 2.0,
    };
    let topic_b = RoadmapTopic {
        id: "topic-b".to_string(),
        title: "Error Handling with thiserror".to_string(),
        description: "Error propagation and custom enums".to_string(),
        difficulty: 0.5,
        prerequisites: vec![],
        estimated_hours: 2.0,
    };

    let topics = vec![topic_a.clone(), topic_b.clone()];

    // Without weakness report, topic-a comes first (lower difficulty)
    let normal_sorted = coach.topological_sort(&topics).unwrap();
    assert_eq!(normal_sorted[0].id, "topic-a");

    // With weakness in Error Handling, topic-b must be dynamically prioritized to index 0!
    let weakness = WeaknessReport {
        weak_areas: vec!["Error Handling [Critical]".to_string()],
        recommended_topics: vec!["Error Handling with thiserror".to_string()],
    };

    let prioritized = coach
        .prioritized_topological_sort(&topics, &weakness)
        .unwrap();
    assert_eq!(
        prioritized[0].id, "topic-b",
        "Weakness topic must be reordered to front when prerequisites allow"
    );
}

#[test]
fn test_coach_schedule_calculator_feasible() {
    let coach = CoachAgent::new();

    let roadmap = Roadmap {
        topics: vec![
            RoadmapTopic {
                id: "t1".to_string(),
                title: "T1".to_string(),
                description: "".to_string(),
                difficulty: 0.3,
                prerequisites: vec![],
                estimated_hours: 4.0,
            },
            RoadmapTopic {
                id: "t2".to_string(),
                title: "T2".to_string(),
                description: "".to_string(),
                difficulty: 0.5,
                prerequisites: vec!["t1".to_string()],
                estimated_hours: 6.0,
            },
        ],
    };

    let profile = UserProfile {
        hours_per_day: 2.0,
        deadline_days: 10,
        ..Default::default()
    };

    let schedule = coach.calculate_schedule(&roadmap, &profile);

    assert_eq!(schedule.total_hours, 10.0);
    assert_eq!(schedule.total_days, 5.0);
    assert!(schedule.feasible);
    assert!(schedule.pace_warning.is_none());
    assert_eq!(schedule.milestones.len(), 2);

    assert_eq!(schedule.milestones[0].cumulative_hours, 4.0);
    assert_eq!(schedule.milestones[0].target_day, 2.0);
    assert!(schedule.milestones[0].within_deadline);

    assert_eq!(schedule.milestones[1].cumulative_hours, 10.0);
    assert_eq!(schedule.milestones[1].target_day, 5.0);
    assert!(schedule.milestones[1].within_deadline);
}

#[test]
fn test_coach_schedule_calculator_exceeding_deadline_triggers_warning() {
    let coach = CoachAgent::new();

    let roadmap = Roadmap {
        topics: vec![RoadmapTopic {
            id: "t1".to_string(),
            title: "Extensive Topic".to_string(),
            description: "".to_string(),
            difficulty: 0.8,
            prerequisites: vec![],
            estimated_hours: 20.0,
        }],
    };

    let profile = UserProfile {
        hours_per_day: 1.0, // 20 days needed
        deadline_days: 7,   // only 7 days available
        ..Default::default()
    };

    let schedule = coach.calculate_schedule(&roadmap, &profile);

    assert_eq!(schedule.total_hours, 20.0);
    assert_eq!(schedule.total_days, 20.0);
    assert!(!schedule.feasible);
    assert!(schedule.pace_warning.is_some());
    assert!(schedule
        .pace_warning
        .unwrap()
        .contains("exceeding the 7-day deadline"));
    assert!(!schedule.milestones[0].within_deadline);
}

#[test]
fn test_coach_roadmap_generation_with_decayed_micro_drills() {
    let coach = CoachAgent::new();
    let profile = UserProfile::default();
    let weakness = WeaknessReport::default();

    let drill = RoadmapTopic {
        id: "micro-drill-borrow-move".to_string(),
        title: "⚡ Micro-Drill: Move Semantics (Spaced Review)".to_string(),
        description: "Decayed concept reinforcement".to_string(),
        difficulty: 0.2,
        prerequisites: vec![],
        estimated_hours: 0.1,
    };

    let roadmap = coach.generate_roadmap_with_decay(&profile, &weakness, vec![drill.clone()]);
    assert!(!roadmap.topics.is_empty());
    // Micro-drill must be prepended as the very first topic
    assert_eq!(roadmap.topics[0].id, "micro-drill-borrow-move");
    assert!(roadmap.topics[0].title.contains("Micro-Drill"));
}

// ============================================================================
// 3. TutorAgent Tests
// ============================================================================

#[test]
fn test_tutor_agent_identity() {
    let agent = TutorAgent::new();
    assert_eq!(agent.id(), "tutor-agent");
    assert_eq!(agent.name(), "Tutor Agent");
}

#[test]
fn test_tutor_provide_material_delivers_high_density_markdown() {
    let tutor = TutorAgent::new();
    let topic = RoadmapTopic {
        id: "topic-ownership".to_string(),
        title: "Memory Safety & RAII Semantics".to_string(),
        description: "Understanding move semantics, ownership transfer, affine types, and Drop."
            .to_string(),
        difficulty: 0.3,
        prerequisites: vec![],
        estimated_hours: 2.5,
    };

    let (material, task) = tutor.provide_material(&topic);

    assert!(material.contains("# Memory Safety & RAII Semantics"));
    assert!(material.contains("```rust"));
    assert!(material.contains("Drop"));
    assert!(material.contains("Checklist"));
    assert!(task.contains("Memory Safety & RAII Semantics"));
    assert!(task.contains("Requirements:"));
}

#[test]
fn test_tutor_revision_tailored_to_compiler_error_e0382() {
    let tutor = TutorAgent::new();
    let topic = RoadmapTopic {
        id: "topic-ownership".to_string(),
        title: "Ownership & Thread Concurrency".to_string(),
        description: "Move semantics across threads.".to_string(),
        difficulty: 0.5,
        prerequisites: vec![],
        estimated_hours: 3.0,
    };

    let (material, task) = tutor.provide_revision_for_compiler_error(&topic, "E0382");

    assert!(material.contains("E0382"));
    assert!(material.contains("Move Semantics"));
    assert!(material.contains("Anti-Pattern"));
    assert!(material.contains("Idiomatic Resolution Pattern"));
    assert!(material.contains("Arc"));
    assert!(material.contains("Socratic Reflection"));
    assert!(task.contains("E0382"));
}

#[test]
fn test_tutor_revision_tailored_to_compiler_error_e0597() {
    let tutor = TutorAgent::new();
    let topic = RoadmapTopic {
        id: "topic-lifetimes".to_string(),
        title: "Borrowing and Scopes".to_string(),
        description: "Reference outliving owner.".to_string(),
        difficulty: 0.6,
        prerequisites: vec![],
        estimated_hours: 3.5,
    };

    let (material, task) = tutor.provide_revision_for_compiler_error(
        &topic,
        "error[E0597]: `local` does not live long enough",
    );

    assert!(material.contains("E0597"));
    assert!(material.contains("Does Not Live Long Enough"));
    assert!(material.contains("Anti-Pattern"));
    assert!(material.contains("Idiomatic Resolution Pattern"));
    assert!(task.contains("E0597"));
}

#[test]
fn test_tutor_all_compiler_guides_valid() {
    assert!(COMPILER_GUIDES.len() >= 6);

    let expected_codes = [
        "E0382", "E0597", "E0502", "E0277", "E0499", "E0308", "E0716",
    ];
    for code in &expected_codes {
        let guide = TutorAgent::lookup_compiler_guide(code);
        assert!(
            guide.is_some(),
            "Guide for {code} must be present in catalog"
        );
        let g = guide.unwrap();
        assert!(!g.anti_pattern.is_empty());
        assert!(!g.idiomatic_solution.is_empty());
        assert!(g.socratic_prompt.contains('?'));
    }
}

// ============================================================================
// 4. ExaminerAgent Tests
// ============================================================================

#[test]
fn test_examiner_agent_identity() {
    let agent = ExaminerAgent::new();
    assert_eq!(agent.id(), "examiner-agent");
    assert_eq!(agent.name(), "Examiner Agent");
}

#[test]
fn test_examiner_dynamic_threshold_formula() {
    // Formula: min(70.0 + difficulty * 15.0, 85.0)
    assert_eq!(ExaminerAgent::compute_pass_threshold(0.0), 70.0);
    assert_eq!(ExaminerAgent::compute_pass_threshold(0.3), 74.5);
    assert_eq!(ExaminerAgent::compute_pass_threshold(0.5), 77.5);
    assert_eq!(ExaminerAgent::compute_pass_threshold(1.0), 85.0);
    assert_eq!(ExaminerAgent::compute_pass_threshold(1.5), 85.0); // Clamped <= 85.0
}

#[test]
fn test_examiner_assess_successful_execution() {
    let examiner = ExaminerAgent::new();
    let topic = RoadmapTopic {
        id: "topic-1".to_string(),
        title: "RAII Pattern".to_string(),
        description: "".to_string(),
        difficulty: 0.5,
        prerequisites: vec![],
        estimated_hours: 2.0,
    };

    let sandbox = SandboxExecutionResult {
        success: true,
        exit_code: 0,
        stdout: "All 12 tests passed successfully.".to_string(),
        stderr: "".to_string(),
        duration_ms: 120,
    };

    let assessment = examiner.assess_execution(&topic, &sandbox);

    assert_eq!(assessment.verdict, Verdict::Pass);
    assert!(assessment.score >= 85.0);
    assert!(assessment.weak_areas.is_empty());
    assert!(assessment.feedback.contains("Mastery verified"));
}

#[test]
fn test_examiner_assess_failed_execution_with_socratic_guidance() {
    let examiner = ExaminerAgent::new();
    let topic = RoadmapTopic {
        id: "topic-tokio".to_string(),
        title: "Tokio Concurrency".to_string(),
        description: "".to_string(),
        difficulty: 0.7,
        prerequisites: vec![],
        estimated_hours: 4.0,
    };

    let sandbox = SandboxExecutionResult {
        success: false,
        exit_code: 101,
        stdout: "".to_string(),
        stderr: "error[E0382]: use of moved value: `tx`\n   --> src/main.rs:14:9".to_string(),
        duration_ms: 350,
    };

    let assessment = examiner.assess_execution(&topic, &sandbox);

    assert_eq!(assessment.verdict, Verdict::Retry);
    assert!(assessment.score < ExaminerAgent::compute_pass_threshold(topic.difficulty));
    assert_eq!(assessment.weak_areas.len(), 1);
    assert!(assessment.weak_areas[0].contains("E0382"));

    // Check Socratic compliance: no code leak, has conceptual questions
    assert!(ExaminerAgent::is_socratic_compliant(
        &assessment.feedback,
        Verdict::Retry
    ));
    assert!(assessment.feedback.contains("Diagnostic Pointer:"));
    assert!(assessment.feedback.contains("Reflective Question:"));
    assert!(!assessment.feedback.contains("```"));
    assert!(!assessment.feedback.contains("fn main"));
}

#[test]
fn test_examiner_socratic_compliance_validator() {
    // Valid Socratic guidance
    let valid_feedback = "Examine the lifetime scopes of your references. Why does the owner drop before the reference is read?";
    assert!(ExaminerAgent::is_socratic_compliant(
        valid_feedback,
        Verdict::Retry
    ));

    // Non-compliant: contains code fences
    let code_fence_feedback = "Here is the fix:\n```rust\nfn main() {}\n```\nDoes this make sense?";
    assert!(!ExaminerAgent::is_socratic_compliant(
        code_fence_feedback,
        Verdict::Retry
    ));

    // Non-compliant: contains direct code implementation without fences
    let direct_code_feedback = "You should write fn foo() and let mut x = 42; What do you think?";
    assert!(!ExaminerAgent::is_socratic_compliant(
        direct_code_feedback,
        Verdict::Retry
    ));

    // Non-compliant: retry feedback lacking reflective question mark
    let no_question_feedback = "The code failed due to lifetime error E0597.";
    assert!(!ExaminerAgent::is_socratic_compliant(
        no_question_feedback,
        Verdict::Retry
    ));
}

// ============================================================================
// 5. Multi-Agent Pipeline Integration Test
// ============================================================================

#[test]
fn test_multi_agent_pedagogical_pipeline() {
    let start_time = Utc::now();

    // 1. Diagnostic phase
    let diag_agent = DiagnosticAgent::new();
    let profile = UserProfile {
        skill: "Rust".to_string(),
        current_level: UserLevel::Intermediate,
        target_level: UserLevel::Advanced,
        hours_per_day: 2.0,
        deadline_days: 14,
        goal: "Master advanced concurrency and memory safety".to_string(),
    };

    let questions = diag_agent.generate_questions(&profile);
    assert_eq!(questions.len(), 6);

    // Learner gets questions 0, 1 correct, fails Tokio question (index 3)
    let answers = vec![
        "1".to_string(), // Ownership: correct
        "1".to_string(), // Borrowing: correct
        "1".to_string(), // Lifetimes: correct
        "2".to_string(), // Tokio: wrong!
        "1".to_string(), // Error Handling: correct
        "1".to_string(), // Trait Bounds: correct
    ];

    let weakness_report = diag_agent.evaluate_answers_for_questions(&questions, &answers);
    assert_eq!(weakness_report.weak_areas.len(), 1);
    assert!(weakness_report.weak_areas[0].contains("Tokio Concurrency"));

    // 2. Coach phase: DAG sorting & dynamic prioritization
    let coach = CoachAgent::new();
    let roadmap = coach.generate_roadmap(&profile, &weakness_report);
    assert!(!roadmap.topics.is_empty());

    // Schedule estimation
    let schedule = coach.calculate_schedule_from_start(&roadmap, &profile, start_time);
    assert!(schedule.total_hours > 0.0);
    assert_eq!(schedule.milestones.len(), roadmap.topics.len());

    // 3. Tutor phase: Generate theory for first milestone
    let tutor = TutorAgent::new();
    let first_topic = &roadmap.topics[0];
    let (theory, challenge) = tutor.provide_material(first_topic);
    assert!(!theory.is_empty());
    assert!(!challenge.is_empty());

    // 4. Examiner phase: Evaluate sandbox failure with Socratic remediation
    let examiner = ExaminerAgent::new();
    let failed_sandbox = SandboxExecutionResult {
        success: false,
        exit_code: 1,
        stdout: "".to_string(),
        stderr: "error[E0597]: `val` does not live long enough".to_string(),
        duration_ms: 400,
    };

    let assessment = examiner.assess_execution(first_topic, &failed_sandbox);
    assert_eq!(assessment.verdict, Verdict::Retry);
    assert!(ExaminerAgent::is_socratic_compliant(
        &assessment.feedback,
        Verdict::Retry
    ));

    // 5. Tutor dynamic revision generation for compiler error
    let (revision_material, revision_task) =
        tutor.provide_revision_for_compiler_error(first_topic, "E0597");
    assert!(revision_material.contains("E0597"));
    assert!(revision_material.contains("Anti-Pattern"));
    assert!(revision_task.contains("E0597"));
}
