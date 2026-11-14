//! Nereus CLI library: unified command line interface, session explorer, and automaton runner.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

#![deny(warnings)]

pub mod args;
pub mod db;
pub mod format;
pub mod session;

pub use args::{Cli, Commands, SessionsCommand};
pub use db::resolve_db_path;
pub use format::{
    format_ascii_table, format_history_table, format_sessions_table, resolve_current_topic,
    summarize_feedback,
};
pub use session::{delete_session_command, history_command, list_sessions_command};

use anyhow::Result;
use nereus_core::models::{Assessment, UserLevel, UserProfile, Verdict};
use nereus_core::state::LearningState;
use nereus_core::traits::StorageBackend;
use nereus_engine::LearningAutomaton;
use nereus_llm::{OpenRouterClient, StubLLMClient};
use nereus_plugin::ModuleRegistry;
use nereus_sandbox::MockSandboxRunner;
use nereus_storage::{AssessmentRecord, SqliteStorage};
use std::sync::Arc;

/// Prints the ASCII banner for the Nereus terminal AI mentor.
pub fn print_banner() {
    println!(
        r#"
                   ▄█▄
                  [***]════════════════════════════════► (Beam of Clarity)
                  █████
                  ░░░░░            _   _ _____ ____  _____ _   _ ____
                  █████           | \ | | ____|  _ \| ____| | | / ___|
                 ░░░░░░░          |  \| |  _| | |_) |  _| | | | \___ \
                █████████         | |\  | |___|  _ <| |___| |_| |___) |
               ▓▓▓▓▓▓▓▓▓▓▓        |_| \_|_____|_| \_\_____|\___/|____/
           ~~~~~~~~~~~~~~~~~~~             T E R M I N A L   A I   M E N T O R
          ~~~~~~~~~~~~~~~~~~~~~                       [ v2.0-rust ]
    "#
    );
}

/// Main entrypoint orchestrating command execution, session discovery, and automaton lifecycle.
pub async fn run(args: Cli) -> Result<()> {
    let db_path = resolve_db_path(args.db.as_deref());
    let storage = Arc::new(SqliteStorage::new(&db_path)?);

    // 1. Handle explicit sessions subcommand if provided
    if let Some(cmd) = &args.command {
        match cmd {
            Commands::Sessions(SessionsCommand::List) => {
                return list_sessions_command(&storage).await;
            }
            Commands::Sessions(SessionsCommand::History { session_id }) => {
                return history_command(&storage, *session_id).await;
            }
            Commands::Sessions(SessionsCommand::Delete { session_id }) => {
                return delete_session_command(&storage, *session_id).await;
            }
        }
    }

    // 2. Mentor session execution (either resume or new session)
    print_banner();

    let llm_client: Arc<dyn nereus_core::traits::LLMClient> = if args.offline {
        println!("🛡️  Mode: Offline Hermetic Stub (Deterministic, Zero API Tokens)");
        Arc::new(StubLLMClient::new())
    } else if let Ok(key) =
        std::env::var("NEREUS_API_KEY").or_else(|_| std::env::var("OPENROUTER_API_KEY"))
    {
        println!("🌐 Mode: Live AI Inference via OpenRouter");
        Arc::new(OpenRouterClient::new(key, None))
    } else {
        println!("⚠️  No API key detected. Falling back to Offline Hermetic Stub mode.");
        Arc::new(StubLLMClient::new())
    };

    let sandbox_runner: Arc<dyn nereus_core::traits::SandboxRunner> =
        Arc::new(MockSandboxRunner::new(false));

    let storage_backend: Arc<dyn StorageBackend> = storage.clone();
    let registry = ModuleRegistry::new(llm_client, sandbox_runner, storage_backend);

    let (mut automaton, is_resumed) = if let Some(sid) = args.session_id {
        println!("💾 Storage: SQLite WAL backend at {}", db_path.display());
        match storage.load_session(sid).await? {
            Some(ctx) => {
                println!("🔄 Resuming learning session: {}", sid);
                (LearningAutomaton::from_context(registry, ctx), true)
            }
            None => {
                anyhow::bail!("Session with ID '{}' not found in database", sid);
            }
        }
    } else if args.resume {
        println!("💾 Storage: SQLite WAL backend at {}", db_path.display());
        let sessions = storage.list_sessions().await?;
        if sessions.is_empty() {
            anyhow::bail!("No saved sessions found in database to resume");
        }
        let most_recent = &sessions[0];
        let ctx = most_recent.context()?;
        println!(
            "🔄 Resuming most recent learning session: {}",
            ctx.session_id
        );
        (LearningAutomaton::from_context(registry, ctx), true)
    } else {
        println!("💾 Storage: SQLite WAL backend at {}", db_path.display());
        println!("⚡ Target Skill: {}", args.skill);
        println!("📊 Learner Level: {}", args.level);
        println!("🎯 Goal: {}", args.goal);

        let parsed_level = match args.level.to_lowercase().as_str() {
            "beginner" => UserLevel::Beginner,
            "advanced" => UserLevel::Advanced,
            _ => UserLevel::Intermediate,
        };

        let profile = UserProfile {
            skill: args.skill.clone(),
            current_level: parsed_level,
            target_level: UserLevel::Advanced,
            hours_per_day: 2.0,
            deadline_days: 14,
            goal: args.goal.clone(),
        };

        let auto = LearningAutomaton::new(registry, profile, args.diagnostic);
        // Persist initial session record so session discovery works immediately
        let _ = storage.save_session(&auto.ctx).await;
        (auto, false)
    };

    if is_resumed {
        println!("⚡ Target Skill: {}", automaton.ctx.profile.skill);
        println!(
            "📊 Learner Level: {:?}",
            automaton.ctx.profile.current_level
        );
        println!("🎯 Goal: {}", automaton.ctx.profile.goal);
        println!("📍 Resumed State: {}", automaton.ctx.state.name());
        if let Some(topic) = automaton.ctx.current_topic() {
            println!("📚 Current Topic: {}", topic.title);
        } else if automaton.ctx.is_completed() {
            println!("🎉 Status: Completed");
        }
    } else {
        println!(
            "\n🌊 Initializing Socratic Learning Automaton [Session: {}]...",
            automaton.ctx.session_id
        );
        println!("📍 Current State: {}\n", automaton.ctx.state.name());
    }

    if args.demo {
        println!("🚀 Running automated lifecycle simulation:\n");
        for step_i in 1..=5 {
            if automaton.ctx.is_completed() {
                println!("🎉 Lifecycle simulation complete! All topics mastered.");
                break;
            }
            let prev_state = automaton.ctx.state.clone();
            println!(
                "--- Step {step_i}: Transitioning from {} ---",
                prev_state.name()
            );
            let next_state = automaton.step().await?;
            println!("    New State: {}\n", next_state.name());

            // If an examination step occurred, record an assessment entry to history
            if let LearningState::Examining { topic, .. } = prev_state {
                let assessment = Assessment {
                    topic_id: topic.id.clone(),
                    score: 90.0,
                    verdict: Verdict::Pass,
                    feedback: "Sandbox assertions verified successfully.".to_string(),
                    weak_areas: vec![],
                };
                let record =
                    AssessmentRecord::from_assessment(automaton.ctx.session_id, &assessment, 1);
                let _ = storage.record_assessment(&record).await;
            }

            if automaton.ctx.is_completed() {
                println!("🎉 Lifecycle simulation complete! All topics mastered.");
                break;
            }
        }
    } else {
        println!("💡 Tip: Pass --demo to simulate state machine progression, or run with Ratatui TUI cockpit.");
    }

    Ok(())
}
