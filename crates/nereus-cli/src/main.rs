//! Nereus 2.0 — The Terminal-First AI Programming Mentor Built in Rust
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use clap::Parser;
use nereus_core::models::{UserLevel, UserProfile};
use nereus_engine::LearningAutomaton;
use nereus_llm::{OpenRouterClient, StubLLMClient};
use nereus_plugin::ModuleRegistry;
use nereus_sandbox::MockSandboxRunner;
use nereus_storage::{MemoryStorage, SqliteStorage};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "nereus",
    version = "0.2.0",
    about = "🌊 The Terminal-First AI Programming Mentor Built in Rust",
    long_about = "Nereus 2.0 is an autonomous AI coding companion engineered in Rust. It diagnoses baseline gaps, generates adaptive roadmaps, and provides strict Socratic feedback using an algebraic state automaton."
)]
pub struct Cli {
    /// Target programming language or technology
    #[arg(short, long, default_value = "Rust")]
    pub skill: String,

    /// Learner baseline level (beginner, intermediate, advanced)
    #[arg(short, long, default_value = "intermediate")]
    pub level: String,

    /// Specific learning goal
    #[arg(
        short,
        long,
        default_value = "Master concurrency and async Rust idioms"
    )]
    pub goal: String,

    /// Force adaptive diagnostic quiz before roadmap compilation
    #[arg(long, default_value_t = false)]
    pub diagnostic: bool,

    /// Run in deterministic offline mode (Zero API keys needed!)
    #[arg(long, default_value_t = false)]
    pub offline: bool,

    /// Resume a previously saved learning session ID
    #[arg(long)]
    pub session_id: Option<String>,

    /// Path to SQLite database file [default: ~/.nereus/nereus.db]
    #[arg(long)]
    pub db: Option<String>,

    /// Run demo automated lifecycle step loop
    #[arg(long, default_value_t = false)]
    pub demo: bool,
}

fn print_banner() {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Cli::parse();
    print_banner();

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

    // 1. Initialize Module Registry with hot-swappable subsystem components
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

    let storage_backend: Arc<dyn nereus_core::traits::StorageBackend> = if let Some(path) = &args.db
    {
        println!("💾 Storage: SQLite WAL backend at {}", path);
        Arc::new(SqliteStorage::new(path)?)
    } else {
        Arc::new(MemoryStorage::new())
    };

    let registry = ModuleRegistry::new(llm_client, sandbox_runner, storage_backend);

    // 2. Initialize the Algebraic FSM
    let mut automaton = LearningAutomaton::new(registry.clone(), profile, args.diagnostic);

    println!(
        "\n🌊 Initializing Socratic Learning Automaton [Session: {}]...",
        automaton.ctx.session_id
    );
    println!("📍 Current State: {}\n", automaton.ctx.state.name());

    if args.demo {
        println!("🚀 Running automated lifecycle simulation:\n");
        for step_i in 1..=5 {
            println!(
                "--- Step {step_i}: Transitioning from {} ---",
                automaton.ctx.state.name()
            );
            let next_state = automaton.step().await?;
            println!("    New State: {}\n", next_state.name());
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
