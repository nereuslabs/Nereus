//! SQLite persistent session store using WAL mode and user_version schema migrations.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::state::StateMachineContext;
use nereus_core::traits::StorageBackend;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::models::{AssessmentRecord, SessionRecord, SessionStore};

/// Current target schema version tracked via `PRAGMA user_version`.
pub const CURRENT_SCHEMA_VERSION: i32 = 2;

/// Declarative schema migration step.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: i32,
    pub description: &'static str,
    pub sql: &'static str,
}

/// Ordered sequence of schema migrations for Nereus 2.0 SQLite persistence.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Create sessions table and indexes",
        sql: r#"
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                user_skill TEXT NOT NULL,
                current_topic_index INTEGER NOT NULL,
                state_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
        "#,
    },
    Migration {
        version: 2,
        description: "Create topic_assessment_log table and indexes with foreign keys",
        sql: r#"
            CREATE TABLE IF NOT EXISTS topic_assessment_log (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                topic_id TEXT NOT NULL,
                score REAL NOT NULL,
                verdict TEXT NOT NULL,
                attempts INTEGER NOT NULL,
                feedback TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions (session_id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_assessment_session ON topic_assessment_log(session_id);
            CREATE INDEX IF NOT EXISTS idx_assessment_topic ON topic_assessment_log(topic_id);
            CREATE INDEX IF NOT EXISTS idx_assessment_timestamp ON topic_assessment_log(timestamp);
        "#,
    },
];

/// Production-grade SQLite storage backend with WAL mode, normalized synchronicity,
/// and incremental schema migration management.
#[derive(Debug, Clone)]
pub struct SqliteStorage {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    /// Opens or creates a SQLite database at the specified path, configures WAL mode,
    /// and runs all pending schema migrations up to `CURRENT_SCHEMA_VERSION`.
    pub fn new(path: impl AsRef<Path>) -> CoreResult<Self> {
        let db_path = path.as_ref().to_path_buf();
        let is_memory = db_path == Path::new(":memory:") || db_path.to_str() == Some(":memory:");

        if !is_memory {
            if let Some(parent) = db_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        CoreError::StorageError(format!(
                            "Failed to create database parent directory '{}': {e}",
                            parent.display()
                        ))
                    })?;
                }
            }
        }

        let mut conn = Connection::open(&db_path)
            .map_err(|e| CoreError::StorageError(format!("Failed to open SQLite database: {e}")))?;

        Self::configure_connection(&conn)?;
        Self::apply_migrations(&mut conn)?;

        Ok(Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Convenience constructor for an ephemeral in-memory SQLite database.
    pub fn in_memory() -> CoreResult<Self> {
        Self::new(":memory:")
    }

    /// Configures connection pragmas: WAL mode, NORMAL synchronous, foreign keys, and busy timeout.
    pub fn configure_connection(conn: &Connection) -> CoreResult<()> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(|e| CoreError::StorageError(format!("Failed to configure SQLite PRAGMAs: {e}")))?;

        Ok(())
    }

    /// Reads the current schema version from `PRAGMA user_version`.
    pub fn get_schema_version(conn: &Connection) -> CoreResult<i32> {
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| {
                CoreError::StorageError(format!("Failed to read PRAGMA user_version: {e}"))
            })
    }

    /// Updates `PRAGMA user_version` to a target version.
    pub fn set_schema_version(conn: &Connection, version: i32) -> CoreResult<()> {
        conn.execute(&format!("PRAGMA user_version = {version};"), [])
            .map_err(|e| {
                CoreError::StorageError(format!(
                    "Failed to set PRAGMA user_version to {version}: {e}"
                ))
            })?;
        Ok(())
    }

    /// Applies all pending migrations up to `CURRENT_SCHEMA_VERSION`.
    pub fn apply_migrations(conn: &mut Connection) -> CoreResult<i32> {
        Self::migrate_to(conn, CURRENT_SCHEMA_VERSION)
    }

    /// Applies pending migrations up to a specific target version.
    pub fn migrate_to(conn: &mut Connection, target_version: i32) -> CoreResult<i32> {
        let current_version = Self::get_schema_version(conn)?;
        if current_version >= target_version {
            tracing::debug!(
                target: "nereus::storage::sqlite",
                current_version,
                target_version,
                "SQLite schema is already at or above target version"
            );
            return Ok(current_version);
        }

        for migration in MIGRATIONS
            .iter()
            .filter(|m| m.version > current_version && m.version <= target_version)
        {
            tracing::info!(
                target: "nereus::storage::sqlite",
                from = current_version,
                to = migration.version,
                description = migration.description,
                "Applying SQLite schema migration"
            );

            let tx = conn.transaction().map_err(|e| {
                CoreError::StorageError(format!("Failed to begin migration transaction: {e}"))
            })?;

            tx.execute_batch(migration.sql).map_err(|e| {
                CoreError::StorageError(format!(
                    "Migration v{} ('{}') execution failed: {e}",
                    migration.version, migration.description
                ))
            })?;

            tx.commit().map_err(|e| {
                CoreError::StorageError(format!(
                    "Failed to commit migration v{} ('{}'): {e}",
                    migration.version, migration.description
                ))
            })?;

            Self::set_schema_version(conn, migration.version)?;
        }

        Self::get_schema_version(conn)
    }

    /// Returns the active database path.
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Retrieves the current `PRAGMA user_version` on the active connection.
    pub fn schema_version(&self) -> CoreResult<i32> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| CoreError::StorageError("SQLite connection lock poisoned".to_string()))?;
        Self::get_schema_version(&conn)
    }

    /// Atomically persists a state machine session context and an optional assessment record
    /// within a single ACID transaction.
    pub async fn save_session_atomic(
        &self,
        ctx: &StateMachineContext,
        assessment: Option<&AssessmentRecord>,
    ) -> CoreResult<()> {
        let state_json =
            serde_json::to_string(ctx).map_err(|e| CoreError::Serialization(e.to_string()))?;
        let session_id = ctx.session_id.to_string();
        let user_skill = ctx.profile.skill.clone();
        let now = chrono::Utc::now().to_rfc3339();

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| CoreError::StorageError("SQLite connection lock poisoned".to_string()))?;

        let tx = conn
            .transaction()
            .map_err(|e| CoreError::StorageError(format!("Failed to begin transaction: {e}")))?;

        tx.execute(
            "INSERT INTO sessions (session_id, user_skill, current_topic_index, state_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(session_id) DO UPDATE SET
                user_skill = excluded.user_skill,
                current_topic_index = excluded.current_topic_index,
                state_json = excluded.state_json,
                updated_at = excluded.updated_at",
            params![
                session_id,
                user_skill,
                ctx.current_topic_index as i64,
                state_json,
                now
            ],
        )
        .map_err(|e| CoreError::StorageError(format!("Failed to upsert session: {e}")))?;

        if let Some(record) = assessment {
            tx.execute(
                "INSERT INTO topic_assessment_log (id, session_id, topic_id, score, verdict, attempts, feedback, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.id,
                    record.session_id.to_string(),
                    record.topic_id,
                    record.score as f64,
                    record.verdict,
                    record.attempts as i64,
                    record.feedback,
                    record.timestamp,
                ],
            )
            .map_err(|e| CoreError::StorageError(format!("Failed to insert assessment log: {e}")))?;
        }

        tx.commit()
            .map_err(|e| CoreError::StorageError(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }

    /// Convenience method to atomically update session state and record an assessment.
    pub async fn save_session_with_assessment(
        &self,
        ctx: &StateMachineContext,
        assessment: &AssessmentRecord,
    ) -> CoreResult<()> {
        self.save_session_atomic(ctx, Some(assessment)).await
    }

    /// Appends a new assessment record to `topic_assessment_log`.
    pub async fn record_assessment(&self, record: &AssessmentRecord) -> CoreResult<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| CoreError::StorageError("SQLite connection lock poisoned".to_string()))?;

        let tx = conn
            .transaction()
            .map_err(|e| CoreError::StorageError(format!("Failed to begin transaction: {e}")))?;

        tx.execute(
            "INSERT INTO topic_assessment_log (id, session_id, topic_id, score, verdict, attempts, feedback, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.session_id.to_string(),
                record.topic_id,
                record.score as f64,
                record.verdict,
                record.attempts as i64,
                record.feedback,
                record.timestamp,
            ],
        )
        .map_err(|e| CoreError::StorageError(format!("Failed to insert assessment record: {e}")))?;

        tx.commit().map_err(|e| {
            CoreError::StorageError(format!("Failed to commit assessment record: {e}"))
        })?;

        Ok(())
    }

    /// Appends multiple assessment records to `topic_assessment_log` inside a single transaction.
    pub async fn record_assessments(&self, records: &[AssessmentRecord]) -> CoreResult<()> {
        if records.is_empty() {
            return Ok(());
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| CoreError::StorageError("SQLite connection lock poisoned".to_string()))?;

        let tx = conn
            .transaction()
            .map_err(|e| CoreError::StorageError(format!("Failed to begin transaction: {e}")))?;

        for record in records {
            tx.execute(
                "INSERT INTO topic_assessment_log (id, session_id, topic_id, score, verdict, attempts, feedback, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.id,
                    record.session_id.to_string(),
                    record.topic_id,
                    record.score as f64,
                    record.verdict,
                    record.attempts as i64,
                    record.feedback,
                    record.timestamp,
                ],
            )
            .map_err(|e| CoreError::StorageError(format!("Failed to insert assessment record: {e}")))?;
        }

        tx.commit().map_err(|e| {
            CoreError::StorageError(format!("Failed to commit batch assessment records: {e}"))
        })?;

        Ok(())
    }

    /// Lists all sessions ordered by most recently updated first.
    pub async fn list_sessions(&self) -> CoreResult<Vec<SessionRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| CoreError::StorageError("SQLite connection lock poisoned".to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT session_id, user_skill, current_topic_index, state_json, updated_at
                 FROM sessions
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| {
                CoreError::StorageError(format!("Failed to prepare list_sessions: {e}"))
            })?;

        let session_rows = stmt
            .query_map([], |row| {
                let sid_str: String = row.get(0)?;
                let user_skill: String = row.get(1)?;
                let current_topic_index: i64 = row.get(2)?;
                let state_json: String = row.get(3)?;
                let updated_at: String = row.get(4)?;
                Ok((
                    sid_str,
                    user_skill,
                    current_topic_index,
                    state_json,
                    updated_at,
                ))
            })
            .map_err(|e| {
                CoreError::StorageError(format!("Failed to execute list_sessions query: {e}"))
            })?;

        let mut results = Vec::new();
        for row_res in session_rows {
            let (sid_str, user_skill, current_topic_index, state_json, updated_at) = row_res
                .map_err(|e| CoreError::StorageError(format!("Row error in list_sessions: {e}")))?;
            let session_id = Uuid::parse_str(&sid_str).map_err(|e| {
                CoreError::StorageError(format!("Corrupt session_id UUID '{sid_str}': {e}"))
            })?;
            results.push(SessionRecord {
                session_id,
                user_skill,
                current_topic_index: current_topic_index as usize,
                state_json,
                updated_at,
            });
        }

        Ok(results)
    }

    /// Retrieves chronological assessment history for a specific session ID.
    pub async fn get_assessment_history(
        &self,
        session_id: Uuid,
    ) -> CoreResult<Vec<AssessmentRecord>> {
        let sid_str = session_id.to_string();
        let conn = self
            .conn
            .lock()
            .map_err(|_| CoreError::StorageError("SQLite connection lock poisoned".to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, topic_id, score, verdict, attempts, feedback, timestamp
                 FROM topic_assessment_log
                 WHERE session_id = ?1
                 ORDER BY timestamp ASC, id ASC",
            )
            .map_err(|e| {
                CoreError::StorageError(format!("Failed to prepare get_assessment_history: {e}"))
            })?;

        let rows = stmt
            .query_map(params![sid_str], |row| {
                let id: String = row.get(0)?;
                let sid_str: String = row.get(1)?;
                let topic_id: String = row.get(2)?;
                let score: f64 = row.get(3)?;
                let verdict: String = row.get(4)?;
                let attempts: i64 = row.get(5)?;
                let feedback: String = row.get(6)?;
                let timestamp: String = row.get(7)?;
                Ok((
                    id, sid_str, topic_id, score, verdict, attempts, feedback, timestamp,
                ))
            })
            .map_err(|e| {
                CoreError::StorageError(format!("Failed to query assessment history: {e}"))
            })?;

        let mut results = Vec::new();
        for row_res in rows {
            let (id, sid_str, topic_id, score, verdict, attempts, feedback, timestamp) = row_res
                .map_err(|e| {
                    CoreError::StorageError(format!("Row error in get_assessment_history: {e}"))
                })?;
            let parsed_sid = Uuid::parse_str(&sid_str).map_err(|e| {
                CoreError::StorageError(format!("Corrupt session_id UUID '{sid_str}': {e}"))
            })?;
            results.push(AssessmentRecord {
                id,
                session_id: parsed_sid,
                topic_id,
                score: score as f32,
                verdict,
                attempts: attempts as usize,
                feedback,
                timestamp,
            });
        }

        Ok(results)
    }

    /// Deletes a session by UUID. If foreign keys are enabled, this cascades to topic_assessment_log.
    pub async fn delete_session(&self, session_id: Uuid) -> CoreResult<bool> {
        let sid_str = session_id.to_string();
        let conn = self
            .conn
            .lock()
            .map_err(|_| CoreError::StorageError("SQLite connection lock poisoned".to_string()))?;

        let rows_affected = conn
            .execute(
                "DELETE FROM sessions WHERE session_id = ?1",
                params![sid_str],
            )
            .map_err(|e| CoreError::StorageError(format!("Failed to delete session: {e}")))?;

        Ok(rows_affected > 0)
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn save_session(&self, ctx: &StateMachineContext) -> CoreResult<()> {
        self.save_session_atomic(ctx, None).await
    }

    async fn load_session(&self, session_id: Uuid) -> CoreResult<Option<StateMachineContext>> {
        let sid_str = session_id.to_string();
        let conn = self
            .conn
            .lock()
            .map_err(|_| CoreError::StorageError("SQLite connection lock poisoned".to_string()))?;

        let mut stmt = conn
            .prepare("SELECT state_json FROM sessions WHERE session_id = ?1")
            .map_err(|e| CoreError::StorageError(e.to_string()))?;

        let mut rows = stmt
            .query(params![sid_str])
            .map_err(|e| CoreError::StorageError(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| CoreError::StorageError(e.to_string()))?
        {
            let json_str: String = row
                .get(0)
                .map_err(|e| CoreError::StorageError(e.to_string()))?;
            let ctx: StateMachineContext = serde_json::from_str(&json_str)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            Ok(Some(ctx))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl SessionStore for SqliteStorage {
    async fn list_sessions(&self) -> CoreResult<Vec<SessionRecord>> {
        self.list_sessions().await
    }

    async fn get_assessment_history(&self, session_id: Uuid) -> CoreResult<Vec<AssessmentRecord>> {
        self.get_assessment_history(session_id).await
    }

    async fn record_assessment(&self, record: &AssessmentRecord) -> CoreResult<()> {
        self.record_assessment(record).await
    }

    async fn save_session_atomic(
        &self,
        ctx: &StateMachineContext,
        assessment: Option<&AssessmentRecord>,
    ) -> CoreResult<()> {
        self.save_session_atomic(ctx, assessment).await
    }

    async fn delete_session(&self, session_id: Uuid) -> CoreResult<bool> {
        self.delete_session(session_id).await
    }
}
