//! Comprehensive integration tests for Nereus Storage:
//! in-memory SQLite, disk SQLite with WAL, and ephemeral MemoryStorage.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use nereus_core::models::{Assessment, UserLevel, UserProfile, Verdict};
use nereus_core::state::{LearningState, StateMachineContext};
use nereus_core::traits::StorageBackend;
use nereus_storage::{
    AssessmentRecord, MemoryStorage, SessionStore, SqliteStorage, CURRENT_SCHEMA_VERSION,
};
use rusqlite::Connection;
use std::sync::Arc;
use uuid::Uuid;

fn create_sample_context(skill: &str) -> StateMachineContext {
    let profile = UserProfile {
        skill: skill.to_string(),
        current_level: UserLevel::Beginner,
        target_level: UserLevel::Intermediate,
        ..Default::default()
    };

    StateMachineContext::new(profile, false)
}

// ============================================================================
// MemoryStorage Tests
// ============================================================================

#[tokio::test]
async fn test_memory_storage_save_and_load() {
    let storage = MemoryStorage::new();
    let ctx = create_sample_context("Rust");
    let session_id = ctx.session_id;

    // Initially absent
    let loaded = storage.load_session(session_id).await.unwrap();
    assert!(loaded.is_none());

    // Save session
    storage.save_session(&ctx).await.unwrap();

    // Load session
    let loaded = storage.load_session(session_id).await.unwrap();
    assert!(loaded.is_some());
    let loaded_ctx = loaded.unwrap();
    assert_eq!(loaded_ctx.session_id, session_id);
    assert_eq!(loaded_ctx.profile.skill, "Rust");
    assert_eq!(loaded_ctx.current_topic_index, 0);
}

#[tokio::test]
async fn test_memory_storage_update_session() {
    let storage = MemoryStorage::new();
    let mut ctx = create_sample_context("Rust");
    let session_id = ctx.session_id;

    storage.save_session(&ctx).await.unwrap();

    // Advance topic index and change state
    ctx.current_topic_index = 2;
    ctx.state = LearningState::Completed;
    storage.save_session(&ctx).await.unwrap();

    let loaded = storage.load_session(session_id).await.unwrap().unwrap();
    assert_eq!(loaded.current_topic_index, 2);
    assert!(matches!(loaded.state, LearningState::Completed));
}

#[tokio::test]
async fn test_memory_storage_list_sessions_ordering() {
    let storage = MemoryStorage::new();
    let ctx1 = create_sample_context("Rust");
    let ctx2 = create_sample_context("Go");
    let ctx3 = create_sample_context("Zig");

    storage.save_session(&ctx1).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    storage.save_session(&ctx2).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    storage.save_session(&ctx3).await.unwrap();

    let list = storage.list_sessions().await.unwrap();
    assert_eq!(list.len(), 3);
    // Ordered updated_at DESC (ctx3 was saved last)
    assert_eq!(list[0].session_id, ctx3.session_id);
    assert_eq!(list[0].user_skill, "Zig");
    assert_eq!(list[1].session_id, ctx2.session_id);
    assert_eq!(list[1].user_skill, "Go");
    assert_eq!(list[2].session_id, ctx1.session_id);
    assert_eq!(list[2].user_skill, "Rust");

    // Verify context deserialization from SessionRecord
    let parsed_ctx = list[0].context().unwrap();
    assert_eq!(parsed_ctx.session_id, ctx3.session_id);
    assert_eq!(parsed_ctx.profile.skill, "Zig");
}

#[tokio::test]
async fn test_memory_storage_assessment_history() {
    let storage = MemoryStorage::new();
    let ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    storage.save_session(&ctx).await.unwrap();

    let a1 = AssessmentRecord::new(sid, "ownership", 65.0, "retry", 1, "Needs work on borrows");
    let a2 = AssessmentRecord::new(
        sid,
        "ownership",
        95.0,
        "pass",
        2,
        "Mastered borrowing & lifetimes",
    );

    storage.record_assessment(&a1).await.unwrap();
    storage.record_assessment(&a2).await.unwrap();

    let history = storage.get_assessment_history(sid).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].topic_id, "ownership");
    assert_eq!(history[0].score, 65.0);
    assert_eq!(history[0].verdict_enum(), Some(Verdict::Retry));
    assert_eq!(history[1].score, 95.0);
    assert_eq!(history[1].verdict_enum(), Some(Verdict::Pass));

    // Assessments for a different session are isolated
    let other_sid = Uuid::new_v4();
    let other_history = storage.get_assessment_history(other_sid).await.unwrap();
    assert!(other_history.is_empty());
}

#[tokio::test]
async fn test_memory_storage_atomic_save_with_assessment() {
    let storage = MemoryStorage::new();
    let ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    let assessment_model = Assessment {
        topic_id: "iterators".to_string(),
        score: 88.5,
        verdict: Verdict::Pass,
        feedback: "Great functional pipeline".to_string(),
        weak_areas: vec![],
    };
    let record = AssessmentRecord::from_assessment(sid, &assessment_model, 1);

    storage
        .save_session_with_assessment(&ctx, &record)
        .await
        .unwrap();

    let loaded = storage.load_session(sid).await.unwrap();
    assert!(loaded.is_some());

    let history = storage.get_assessment_history(sid).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].topic_id, "iterators");
    assert_eq!(history[0].score, 88.5);
    assert_eq!(history[0].verdict, "pass");
}

#[tokio::test]
async fn test_memory_storage_delete_session_cascades() {
    let storage = MemoryStorage::new();
    let ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    let record = AssessmentRecord::new(sid, "generics", 100.0, "pass", 1, "Flawless");
    storage
        .save_session_with_assessment(&ctx, &record)
        .await
        .unwrap();

    assert_eq!(storage.len().await, 1);
    assert!(!storage.is_empty().await);

    let deleted = storage.delete_session(sid).await.unwrap();
    assert!(deleted);

    assert_eq!(storage.len().await, 0);
    assert!(storage.is_empty().await);
    assert!(storage.load_session(sid).await.unwrap().is_none());
    assert!(storage
        .get_assessment_history(sid)
        .await
        .unwrap()
        .is_empty());

    // Deleting again returns false
    let deleted_again = storage.delete_session(sid).await.unwrap();
    assert!(!deleted_again);
}

#[tokio::test]
async fn test_memory_storage_clone_shared_state() {
    let s1 = MemoryStorage::new();
    let s2 = s1.clone();

    let ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    s1.save_session(&ctx).await.unwrap();
    let loaded = s2.load_session(sid).await.unwrap();
    assert!(loaded.is_some());
}

// ============================================================================
// SqliteStorage Migration Engine Tests
// ============================================================================

#[test]
fn test_sqlite_migration_engine_step_by_step() {
    let mut conn = Connection::open_in_memory().unwrap();
    SqliteStorage::configure_connection(&conn).unwrap();

    // Initial user_version must be 0
    let v0 = SqliteStorage::get_schema_version(&conn).unwrap();
    assert_eq!(v0, 0);

    // Migrate to v1 (creates sessions table)
    let v1 = SqliteStorage::migrate_to(&mut conn, 1).unwrap();
    assert_eq!(v1, 1);
    assert_eq!(SqliteStorage::get_schema_version(&conn).unwrap(), 1);

    // Verify sessions table exists
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 1);

    // Verify topic_assessment_log does not exist yet at v1
    let log_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='topic_assessment_log'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(log_count, 0);

    // Migrate to v2 (creates topic_assessment_log)
    let v2 = SqliteStorage::migrate_to(&mut conn, 2).unwrap();
    assert_eq!(v2, 2);
    assert_eq!(SqliteStorage::get_schema_version(&conn).unwrap(), 2);

    let log_count_v2: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='topic_assessment_log'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(log_count_v2, 1);

    // Applying migrations again is idempotent
    let idempotent_v = SqliteStorage::apply_migrations(&mut conn).unwrap();
    assert_eq!(idempotent_v, CURRENT_SCHEMA_VERSION);
    assert_eq!(
        SqliteStorage::get_schema_version(&conn).unwrap(),
        CURRENT_SCHEMA_VERSION
    );
}

#[tokio::test]
async fn test_sqlite_in_memory_initialization() {
    let storage = SqliteStorage::in_memory().unwrap();
    let version = storage.schema_version().unwrap();
    assert_eq!(version, CURRENT_SCHEMA_VERSION);
}

// ============================================================================
// SqliteStorage CRUD & Atomic Transaction Tests
// ============================================================================

#[tokio::test]
async fn test_sqlite_save_and_load_session() {
    let storage = SqliteStorage::in_memory().unwrap();
    let ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    // Verify non-existent session returns None
    assert!(storage.load_session(sid).await.unwrap().is_none());

    // Save session
    storage.save_session(&ctx).await.unwrap();

    // Load and verify
    let loaded = storage
        .load_session(sid)
        .await
        .unwrap()
        .expect("session must exist");
    assert_eq!(loaded.session_id, sid);
    assert_eq!(loaded.profile.skill, "Rust");
    assert_eq!(loaded.profile.current_level, UserLevel::Beginner);
    assert_eq!(loaded.current_topic_index, 0);
}

#[tokio::test]
async fn test_sqlite_update_session_upsert() {
    let storage = SqliteStorage::in_memory().unwrap();
    let mut ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    storage.save_session(&ctx).await.unwrap();

    ctx.current_topic_index = 5;
    ctx.profile.skill = "Rust Async".to_string();
    ctx.state = LearningState::Completed;
    storage.save_session(&ctx).await.unwrap();

    let loaded = storage.load_session(sid).await.unwrap().unwrap();
    assert_eq!(loaded.current_topic_index, 5);
    assert_eq!(loaded.profile.skill, "Rust Async");
    assert!(matches!(loaded.state, LearningState::Completed));

    // Confirm sessions table has exactly 1 row
    let list = storage.list_sessions().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].user_skill, "Rust Async");
}

#[tokio::test]
async fn test_sqlite_list_sessions_ordering() {
    let storage = SqliteStorage::in_memory().unwrap();
    let ctx1 = create_sample_context("Rust");
    let ctx2 = create_sample_context("TypeScript");
    let ctx3 = create_sample_context("Elixir");

    storage.save_session(&ctx1).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
    storage.save_session(&ctx2).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
    storage.save_session(&ctx3).await.unwrap();

    let list = storage.list_sessions().await.unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].session_id, ctx3.session_id);
    assert_eq!(list[0].user_skill, "Elixir");
    assert_eq!(list[1].session_id, ctx2.session_id);
    assert_eq!(list[1].user_skill, "TypeScript");
    assert_eq!(list[2].session_id, ctx1.session_id);
    assert_eq!(list[2].user_skill, "Rust");
}

#[tokio::test]
async fn test_sqlite_assessment_history_and_batch() {
    let storage = SqliteStorage::in_memory().unwrap();
    let ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    // Save session first to satisfy foreign key constraint
    storage.save_session(&ctx).await.unwrap();

    let r1 = AssessmentRecord::with_id(
        "rec-01",
        sid,
        "macros",
        70.0,
        "retry",
        1,
        "Macro syntax incomplete",
        "2026-11-09T10:00:00Z",
    );
    let r2 = AssessmentRecord::with_id(
        "rec-02",
        sid,
        "macros",
        95.0,
        "pass",
        2,
        "Procedural macro fully functional",
        "2026-11-09T10:15:00Z",
    );

    storage.record_assessments(&[r1, r2]).await.unwrap();

    let history = storage.get_assessment_history(sid).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].id, "rec-01");
    assert_eq!(history[0].score, 70.0);
    assert_eq!(history[0].attempts, 1);
    assert_eq!(history[1].id, "rec-02");
    assert_eq!(history[1].score, 95.0);
    assert_eq!(history[1].attempts, 2);
}

#[tokio::test]
async fn test_sqlite_atomic_save_session_with_assessment() {
    let storage = SqliteStorage::in_memory().unwrap();
    let ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    let record = AssessmentRecord::new(
        sid,
        "pinning",
        91.0,
        "pass",
        1,
        "Understands Unpin contract",
    );

    // Atomically persist session + assessment
    storage
        .save_session_with_assessment(&ctx, &record)
        .await
        .unwrap();

    let loaded = storage.load_session(sid).await.unwrap();
    assert!(loaded.is_some());

    let history = storage.get_assessment_history(sid).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].topic_id, "pinning");
    assert_eq!(history[0].score, 91.0);
    assert_eq!(history[0].verdict, "pass");
}

#[tokio::test]
async fn test_sqlite_foreign_key_enforcement() {
    let storage = SqliteStorage::in_memory().unwrap();
    let non_existent_sid = Uuid::new_v4();

    let record = AssessmentRecord::new(
        non_existent_sid,
        "orphaned",
        50.0,
        "retry",
        1,
        "Should fail foreign key check",
    );

    // Attempting to record an assessment for a non-existent session must fail due to foreign keys
    let result = storage.record_assessment(&record).await;
    assert!(
        result.is_err(),
        "Foreign key check should reject orphaned assessment"
    );
}

#[tokio::test]
async fn test_sqlite_cascade_delete() {
    let storage = SqliteStorage::in_memory().unwrap();
    let ctx = create_sample_context("Rust");
    let sid = ctx.session_id;

    let record = AssessmentRecord::new(sid, "unsafe", 85.0, "pass", 1, "Sound pointer arithmetic");
    storage
        .save_session_with_assessment(&ctx, &record)
        .await
        .unwrap();

    assert_eq!(storage.list_sessions().await.unwrap().len(), 1);
    assert_eq!(storage.get_assessment_history(sid).await.unwrap().len(), 1);

    // Delete session
    let deleted = storage.delete_session(sid).await.unwrap();
    assert!(deleted);

    // Session and assessment history must be gone due to ON DELETE CASCADE
    assert_eq!(storage.list_sessions().await.unwrap().len(), 0);
    assert_eq!(storage.get_assessment_history(sid).await.unwrap().len(), 0);
}

// ============================================================================
// On-Disk SQLite WAL Mode & Persistence Across Connections Tests
// ============================================================================

#[tokio::test]
async fn test_sqlite_disk_wal_and_reopen_persistence() {
    let temp_dir = std::env::temp_dir();
    let db_filename = format!("nereus_test_wal_{}.db", Uuid::new_v4());
    let db_path = temp_dir.join(db_filename);

    let ctx = create_sample_context("Rust High Perf");
    let sid = ctx.session_id;

    // 1. First connection: write data to disk
    {
        let storage = SqliteStorage::new(&db_path).unwrap();
        assert_eq!(storage.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);

        let record = AssessmentRecord::new(sid, "simd", 98.0, "pass", 1, "AVX-512 optimized");
        storage
            .save_session_with_assessment(&ctx, &record)
            .await
            .unwrap();
    }

    // 2. Second connection: reopen existing database file
    {
        let storage2 = SqliteStorage::new(&db_path).unwrap();
        let loaded = storage2
            .load_session(sid)
            .await
            .unwrap()
            .expect("must persist across instances");
        assert_eq!(loaded.session_id, sid);
        assert_eq!(loaded.profile.skill, "Rust High Perf");

        let history = storage2.get_assessment_history(sid).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].topic_id, "simd");
        assert_eq!(history[0].score, 98.0);

        let sessions = storage2.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, sid);
    }

    // Clean up temporary test files
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
}

// ============================================================================
// Dynamic Trait Object (SessionStore) Polymorphism Tests
// ============================================================================

#[tokio::test]
async fn test_session_store_trait_polymorphism() {
    let memory_store: Arc<dyn SessionStore> = Arc::new(MemoryStorage::new());
    let sqlite_store: Arc<dyn SessionStore> = Arc::new(SqliteStorage::in_memory().unwrap());

    for store in &[memory_store, sqlite_store] {
        let ctx = create_sample_context("PolySkill");
        let sid = ctx.session_id;

        store.save_session(&ctx).await.unwrap();
        let loaded = store.load_session(sid).await.unwrap();
        assert!(loaded.is_some());

        let record =
            AssessmentRecord::new(sid, "polymorphism", 90.0, "pass", 1, "Clean abstraction");
        store.record_assessment(&record).await.unwrap();

        let history = store.get_assessment_history(sid).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].topic_id, "polymorphism");

        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].user_skill, "PolySkill");

        store.delete_session(sid).await.unwrap();
        assert_eq!(store.list_sessions().await.unwrap().len(), 0);
    }
}
