//! Command-line argument definitions and subcommands for Nereus CLI.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use clap::{Parser, Subcommand};
use uuid::Uuid;

/// Main command-line arguments for Nereus 2.0.
#[derive(Parser, Debug, Clone)]
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

    /// Automatically load the most recently updated session from storage and resume
    #[arg(long, default_value_t = false)]
    pub resume: bool,

    /// Resume a specific learning session by UUID
    #[arg(long)]
    pub session_id: Option<Uuid>,

    /// Path to SQLite database file [default: ~/.nereus/nereus.db]
    #[arg(long, global = true)]
    pub db: Option<String>,

    /// Run demo automated lifecycle step loop
    #[arg(long, default_value_t = false)]
    pub demo: bool,

    /// Subcommand dispatch
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available top-level subcommands.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    /// Manage stored learning sessions and assessment history
    #[command(subcommand, alias = "session")]
    Sessions(SessionsCommand),
}

/// Subcommands under `nereus sessions`.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum SessionsCommand {
    /// Print a formatted ASCII table listing all stored sessions
    List,

    /// Print chronological assessment history for a specific session
    History {
        /// Session UUID to query
        session_id: Uuid,
    },

    /// Delete a stored session and its assessment history
    Delete {
        /// Session UUID to delete
        session_id: Uuid,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cli_args() {
        let cli = Cli::parse_from(["nereus"]);
        assert_eq!(cli.skill, "Rust");
        assert_eq!(cli.level, "intermediate");
        assert_eq!(cli.goal, "Master concurrency and async Rust idioms");
        assert!(!cli.diagnostic);
        assert!(!cli.offline);
        assert!(!cli.resume);
        assert!(cli.session_id.is_none());
        assert!(cli.db.is_none());
        assert!(!cli.demo);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_resume_flag() {
        let cli = Cli::parse_from(["nereus", "--resume"]);
        assert!(cli.resume);
        assert!(cli.session_id.is_none());
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_session_id_flag() {
        let id_str = "550e8400-e29b-41d4-a716-446655440000";
        let cli = Cli::parse_from(["nereus", "--session-id", id_str]);
        assert_eq!(cli.session_id, Some(Uuid::parse_str(id_str).unwrap()));
        assert!(!cli.resume);
    }

    #[test]
    fn test_sessions_list_subcommand() {
        let cli = Cli::parse_from(["nereus", "sessions", "list"]);
        assert_eq!(cli.command, Some(Commands::Sessions(SessionsCommand::List)));
    }

    #[test]
    fn test_session_alias_list() {
        let cli = Cli::parse_from(["nereus", "session", "list"]);
        assert_eq!(cli.command, Some(Commands::Sessions(SessionsCommand::List)));
    }

    #[test]
    fn test_sessions_history_subcommand() {
        let id_str = "550e8400-e29b-41d4-a716-446655440000";
        let cli = Cli::parse_from(["nereus", "sessions", "history", id_str]);
        assert_eq!(
            cli.command,
            Some(Commands::Sessions(SessionsCommand::History {
                session_id: Uuid::parse_str(id_str).unwrap(),
            }))
        );
    }

    #[test]
    fn test_sessions_delete_subcommand() {
        let id_str = "550e8400-e29b-41d4-a716-446655440000";
        let cli = Cli::parse_from(["nereus", "sessions", "delete", id_str]);
        assert_eq!(
            cli.command,
            Some(Commands::Sessions(SessionsCommand::Delete {
                session_id: Uuid::parse_str(id_str).unwrap(),
            }))
        );
    }

    #[test]
    fn test_global_db_flag() {
        let cli = Cli::parse_from(["nereus", "--db", "/tmp/test.db", "sessions", "list"]);
        assert_eq!(cli.db, Some("/tmp/test.db".to_string()));
        assert_eq!(cli.command, Some(Commands::Sessions(SessionsCommand::List)));

        let cli2 = Cli::parse_from(["nereus", "sessions", "list", "--db", "/tmp/test.db"]);
        assert_eq!(cli2.db, Some("/tmp/test.db".to_string()));
        assert_eq!(
            cli2.command,
            Some(Commands::Sessions(SessionsCommand::List))
        );
    }
}
