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

    /// Run the automated Socratic Mentor LLM Benchmark Suite
    Bench(BenchArgs),

    /// Search historical topic assessments and feedback using full-text search
    Search(SearchArgs),

    /// Synchronize learning journals and assessment logs with a local or remote Git repository
    Sync(SyncArgs),
}

/// Arguments for `nereus search`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct SearchArgs {
    /// Search query or compiler error code
    pub query: String,

    /// Maximum number of matching assessment entries to display [default: 10]
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,
}

/// Arguments for `nereus sync`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct SyncArgs {
    /// Target directory for Git learning journal [default: ~/.nereus/journal]
    #[arg(short, long)]
    pub path: Option<String>,

    /// Git remote URL to push commits to (optional)
    #[arg(short, long)]
    pub remote: Option<String>,

    /// Custom commit message for journal updates [default: "chore(sync): update Nereus learning journal"]
    #[arg(
        short,
        long,
        default_value = "chore(sync): update Nereus learning journal"
    )]
    pub message: String,

    /// Dry run: render and display journal files without committing or pushing
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Arguments for `nereus bench`.
#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct BenchArgs {
    /// LLM provider or model identifier to benchmark [default: "stub"]
    #[arg(short, long, default_value = "stub")]
    pub model: String,

    /// Force deterministic offline benchmark execution via StubLLMClient
    #[arg(long, default_value_t = false)]
    pub offline: bool,

    /// Output format for benchmark report (markdown or json) [default: "markdown"]
    #[arg(short, long, default_value = "markdown")]
    pub format: String,

    /// Optional file path to export benchmark report
    #[arg(short, long)]
    pub export: Option<String>,
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

    /// Export a learning session to Markdown, Asciinema .cast, or SVG format
    Export {
        /// Session UUID to export
        session_id: Uuid,

        /// Output format (markdown, cast, or svg) [default: markdown]
        #[arg(short, long, default_value = "markdown")]
        format: String,

        /// Optional output file path
        #[arg(short, long)]
        output: Option<String>,
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

    #[test]
    fn test_bench_subcommand_args() {
        let cli = Cli::parse_from(["nereus", "bench", "--offline", "--format", "json"]);
        assert_eq!(
            cli.command,
            Some(Commands::Bench(BenchArgs {
                model: "stub".to_string(),
                offline: true,
                format: "json".to_string(),
                export: None,
            }))
        );
    }

    #[test]
    fn test_sessions_export_subcommand() {
        let id_str = "550e8400-e29b-41d4-a716-446655440000";
        let cli = Cli::parse_from([
            "nereus",
            "sessions",
            "export",
            id_str,
            "--format",
            "cast",
            "--output",
            "/tmp/out.cast",
        ]);
        assert_eq!(
            cli.command,
            Some(Commands::Sessions(SessionsCommand::Export {
                session_id: Uuid::parse_str(id_str).unwrap(),
                format: "cast".to_string(),
                output: Some("/tmp/out.cast".to_string()),
            }))
        );
    }

    #[test]
    fn test_search_subcommand_args() {
        let cli = Cli::parse_from(["nereus", "search", "borrow checker", "--limit", "5"]);
        assert_eq!(
            cli.command,
            Some(Commands::Search(SearchArgs {
                query: "borrow checker".to_string(),
                limit: 5,
            }))
        );
    }

    #[test]
    fn test_sync_subcommand_args() {
        let cli = Cli::parse_from([
            "nereus",
            "sync",
            "--path",
            "/tmp/journal",
            "--message",
            "feat: sync notes",
            "--dry-run",
        ]);
        assert_eq!(
            cli.command,
            Some(Commands::Sync(SyncArgs {
                path: Some("/tmp/journal".to_string()),
                remote: None,
                message: "feat: sync notes".to_string(),
                dry_run: true,
            }))
        );
    }
}
