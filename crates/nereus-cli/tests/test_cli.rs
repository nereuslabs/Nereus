//! End-to-end integration tests for Nereus CLI session resume and explorer commands.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use clap::Parser;
use nereus_cli::{run, Cli};
use nereus_storage::SqliteStorage;
use uuid::Uuid;

fn temp_db_path() -> String {
    let filename = format!("nereus_test_cli_{}.db", Uuid::new_v4());
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
async fn test_cli_new_session_and_demo_execution() {
    let db = temp_db_path();
    let cli = Cli::parse_from([
        "nereus",
        "--skill",
        "Rust",
        "--offline",
        "--demo",
        "--db",
        &db,
    ]);

    let res = run(cli).await;
    assert!(res.is_ok(), "CLI run failed: {:?}", res);

    // Verify session was persisted to SQLite
    let storage = SqliteStorage::new(&db).unwrap();
    let sessions = storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].user_skill, "Rust");

    // Verify assessment history was logged during demo
    let history = storage
        .get_assessment_history(sessions[0].session_id)
        .await
        .unwrap();
    assert!(!history.is_empty());
    assert_eq!(history[0].topic_id, "topic-1");
    assert_eq!(history[0].verdict, "pass");

    cleanup_db(&db);
}

#[tokio::test]
async fn test_cli_sessions_list_command() {
    let db = temp_db_path();

    // 1. Initial list on empty database
    let list_cli = Cli::parse_from(["nereus", "--db", &db, "sessions", "list"]);
    assert!(run(list_cli).await.is_ok());

    // 2. Populate with two sessions
    let run1 = Cli::parse_from(["nereus", "--skill", "Go", "--offline", "--db", &db]);
    run(run1).await.unwrap();

    let run2 = Cli::parse_from(["nereus", "--skill", "Rust", "--offline", "--db", &db]);
    run(run2).await.unwrap();

    // 3. List command with records
    let list_cli2 = Cli::parse_from(["nereus", "sessions", "list", "--db", &db]);
    assert!(run(list_cli2).await.is_ok());

    let storage = SqliteStorage::new(&db).unwrap();
    let sessions = storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 2);

    cleanup_db(&db);
}

#[tokio::test]
async fn test_cli_sessions_history_and_delete() {
    let db = temp_db_path();

    // Seed session with demo assessments
    let run_cli = Cli::parse_from([
        "nereus",
        "--skill",
        "Rust",
        "--offline",
        "--demo",
        "--db",
        &db,
    ]);
    run(run_cli).await.unwrap();

    let storage = SqliteStorage::new(&db).unwrap();
    let sessions = storage.list_sessions().await.unwrap();
    let sid = sessions[0].session_id;

    // History command
    let hist_cli = Cli::parse_from([
        "nereus",
        "sessions",
        "history",
        &sid.to_string(),
        "--db",
        &db,
    ]);
    assert!(run(hist_cli).await.is_ok());

    // History for non-existent session
    let random_id = Uuid::new_v4().to_string();
    let hist_missing = Cli::parse_from(["nereus", "sessions", "history", &random_id, "--db", &db]);
    assert!(run(hist_missing).await.is_ok());

    // Delete session
    let del_cli = Cli::parse_from([
        "nereus",
        "sessions",
        "delete",
        &sid.to_string(),
        "--db",
        &db,
    ]);
    assert!(run(del_cli).await.is_ok());

    // Verify session and history are gone
    let sessions_after = storage.list_sessions().await.unwrap();
    assert!(sessions_after.is_empty());
    let history_after = storage.get_assessment_history(sid).await.unwrap();
    assert!(history_after.is_empty());

    // Delete non-existent session
    let del_again = Cli::parse_from([
        "nereus",
        "sessions",
        "delete",
        &sid.to_string(),
        "--db",
        &db,
    ]);
    assert!(run(del_again).await.is_ok());

    cleanup_db(&db);
}

#[tokio::test]
async fn test_cli_resume_most_recent_session() {
    let db = temp_db_path();

    // Session 1: Skill Python
    let run1 = Cli::parse_from(["nereus", "--skill", "Python", "--offline", "--db", &db]);
    run(run1).await.unwrap();

    // Small delay to ensure timestamp differs
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Session 2: Skill Rust
    let run2 = Cli::parse_from(["nereus", "--skill", "Rust", "--offline", "--db", &db]);
    run(run2).await.unwrap();

    // Resume most recent (should be Rust session)
    let resume_cli = Cli::parse_from(["nereus", "--resume", "--offline", "--demo", "--db", &db]);
    let res = run(resume_cli).await;
    assert!(res.is_ok(), "Resume failed: {:?}", res);

    cleanup_db(&db);
}

#[tokio::test]
async fn test_cli_resume_specific_session_id() {
    let db = temp_db_path();

    // Session 1: Skill Go
    let run1 = Cli::parse_from(["nereus", "--skill", "Go", "--offline", "--db", &db]);
    run(run1).await.unwrap();

    let storage = SqliteStorage::new(&db).unwrap();
    let sessions = storage.list_sessions().await.unwrap();
    let session_1_id = sessions[0].session_id;

    // Session 2: Skill Rust
    let run2 = Cli::parse_from(["nereus", "--skill", "Rust", "--offline", "--db", &db]);
    run(run2).await.unwrap();

    // Explicitly resume Session 1 by UUID
    let resume_cli = Cli::parse_from([
        "nereus",
        "--session-id",
        &session_1_id.to_string(),
        "--offline",
        "--demo",
        "--db",
        &db,
    ]);
    let res = run(resume_cli).await;
    assert!(res.is_ok(), "Resume by session-id failed: {:?}", res);

    cleanup_db(&db);
}

#[tokio::test]
async fn test_cli_resume_errors() {
    let db = temp_db_path();

    // 1. Resume on empty database returns error
    let resume_empty = Cli::parse_from(["nereus", "--resume", "--offline", "--db", &db]);
    let res1 = run(resume_empty).await;
    assert!(res1.is_err());
    assert!(res1
        .unwrap_err()
        .to_string()
        .contains("No saved sessions found"));

    // 2. Specific session ID not found
    let random_id = Uuid::new_v4().to_string();
    let resume_missing = Cli::parse_from([
        "nereus",
        "--session-id",
        &random_id,
        "--offline",
        "--db",
        &db,
    ]);
    let res2 = run(resume_missing).await;
    assert!(res2.is_err());
    assert!(res2
        .unwrap_err()
        .to_string()
        .contains("not found in database"));

    cleanup_db(&db);
}

#[tokio::test]
async fn test_cli_hot_switch_providers_integration() {
    use nereus_cli::{switch_llm_provider, switch_sandbox_runner};
    use nereus_llm::StubLLMClient;
    use nereus_plugin::ModuleRegistry;
    use nereus_sandbox::MockSandboxRunner;
    use std::sync::Arc;

    let db = temp_db_path();
    let storage = Arc::new(SqliteStorage::new(&db).unwrap());
    let registry = ModuleRegistry::new(
        Arc::new(StubLLMClient::new()),
        Arc::new(MockSandboxRunner::new(false)),
        storage,
    );

    // 1. Hot-switch LLM provider
    let switched = switch_llm_provider(&registry, "stub").await.unwrap();
    assert_eq!(switched, "stub");

    let switched = switch_llm_provider(&registry, "openrouter").await.unwrap();
    assert_eq!(switched, "openrouter");

    let switched = switch_llm_provider(&registry, "anthropic").await.unwrap();
    assert_eq!(switched, "anthropic");

    let switched = switch_llm_provider(&registry, "ollama").await.unwrap();
    assert_eq!(switched, "ollama");

    // 2. Hot-switch Sandbox runner
    let s_switched = switch_sandbox_runner(&registry, "mock").await.unwrap();
    assert_eq!(s_switched, "mock");

    let s_switched = switch_sandbox_runner(&registry, "host").await.unwrap();
    assert_eq!(s_switched, "host");

    let s_switched = switch_sandbox_runner(&registry, "wasmtime").await.unwrap();
    assert_eq!(s_switched, "wasmtime");

    cleanup_db(&db);
}

#[tokio::test]
async fn test_cli_bench_subcommand_e2e() {
    let cli = Cli::parse_from(["nereus", "bench", "--offline", "--format", "json"]);
    let res = run(cli).await;
    assert!(res.is_ok(), "Benchmark command failed: {:?}", res);
}

#[tokio::test]
async fn test_cli_sessions_export_e2e() {
    let db = temp_db_path();

    // 1. Run a session to persist context and assessments
    let init_cli = Cli::parse_from([
        "nereus",
        "--skill",
        "Rust Concurrency",
        "--offline",
        "--demo",
        "--db",
        &db,
    ]);
    run(init_cli).await.unwrap();

    let storage = SqliteStorage::new(&db).unwrap();
    let sessions = storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);
    let session_id = sessions[0].session_id;

    // 2. Export Markdown to file
    let md_path = std::env::temp_dir()
        .join(format!("test_export_{}.md", session_id))
        .to_string_lossy()
        .to_string();
    let export_md = Cli::parse_from([
        "nereus",
        "--db",
        &db,
        "sessions",
        "export",
        &session_id.to_string(),
        "--format",
        "markdown",
        "--output",
        &md_path,
    ]);
    let res_md = run(export_md).await;
    assert!(res_md.is_ok());
    assert!(std::path::Path::new(&md_path).exists());
    let md_content = std::fs::read_to_string(&md_path).unwrap();
    assert!(md_content.contains("# 🌊 Nereus Learning Session Report"));
    assert!(md_content.contains("Rust Concurrency"));
    let _ = std::fs::remove_file(&md_path);

    // 3. Export Asciinema Cast to stdout
    let export_cast = Cli::parse_from([
        "nereus",
        "--db",
        &db,
        "sessions",
        "export",
        &session_id.to_string(),
        "--format",
        "cast",
    ]);
    let res_cast = run(export_cast).await;
    assert!(res_cast.is_ok());

    // 4. Export SVG
    let svg_path = std::env::temp_dir()
        .join(format!("test_export_{}.svg", session_id))
        .to_string_lossy()
        .to_string();
    let export_svg = Cli::parse_from([
        "nereus",
        "--db",
        &db,
        "sessions",
        "export",
        &session_id.to_string(),
        "--format",
        "svg",
        "--output",
        &svg_path,
    ]);
    let res_svg = run(export_svg).await;
    assert!(res_svg.is_ok());
    assert!(std::path::Path::new(&svg_path).exists());
    let svg_content = std::fs::read_to_string(&svg_path).unwrap();
    assert!(svg_content.contains("<svg"));
    assert!(svg_content.contains("</svg>"));
    let _ = std::fs::remove_file(&svg_path);

    cleanup_db(&db);
}
