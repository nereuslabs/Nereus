//! Nereus 2.0 — The Terminal-First AI Programming Mentor Built in Rust
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use clap::Parser;

pub mod engine;
pub mod models;
pub mod agents;
pub mod llm;

#[derive(Parser, Debug)]
#[command(
    name = "nereus",
    version = "0.3.0",
    about = "🌊 The Terminal-First AI Programming Mentor Built in Rust",
    long_about = "Nereus is an autonomous AI coding companion that diagnoses skill gaps, generates adaptive roadmaps, and provides Socratic feedback using an algebraic state automaton."
)]
pub struct Cli {
    /// Target programming language or technology
    #[arg(short, long, default_value = "Rust")]
    pub skill: String,

    /// Learner baseline level (beginner, intermediate, advanced)
    #[arg(short, long, default_value = "intermediate")]
    pub level: String,

    /// Specific learning goal
    #[arg(short, long, default_value = "Master concurrency and async Rust")]
    pub goal: String,

    /// Force adaptive diagnostic quiz before roadmap generation
    #[arg(long, default_value_t = false)]
    pub diagnostic: bool,

    /// Run in deterministic offline mode (Zero API keys needed!)
    #[arg(long, default_value_t = false)]
    pub offline: bool,

    /// Resume a previously saved learning session ID
    #[arg(long)]
    pub session_id: Option<String>,
}

fn print_banner() {
    println!(r#"
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
    "#);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    print_banner();

    println!("⚡ Target Skill: {}", args.skill);
    println!("📊 Learner Level: {}", args.level);
    println!("🎯 Goal: {}", args.goal);

    if args.offline {
        println!("🛡️  Mode: Offline Hermetic Stub (Deterministic)");
    } else {
        println!("🌐 Mode: Live AI Inference");
    }

    println!("\nStarting Socratic Learning Automaton...");
    Ok(())
}
