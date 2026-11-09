//! SQLite persistent session store using WAL mode.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::state::StateMachineContext;
use nereus_core::traits::StorageBackend;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SqliteStorage {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    pub fn new(path: impl AsRef<Path>) -> CoreResult<Self> {
        let db_path = path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| CoreError::StorageError(e.to_string()))?;

        // Enable WAL mode for high concurrency
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                user_skill TEXT NOT NULL,
                current_topic_index INTEGER NOT NULL,
                state_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
        ").map_err(|e| CoreError::StorageError(e.to_string()))?;

        Ok(Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn save_session(&self, ctx: &StateMachineContext) -> CoreResult<()> {
        let state_json = serde_json::to_string(ctx)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        let session_id = ctx.session_id.to_string();
        let user_skill = ctx.profile.skill.clone();
        let now = chrono::Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (session_id, user_skill, current_topic_index, state_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(session_id) DO UPDATE SET
                current_topic_index = excluded.current_topic_index,
                state_json = excluded.state_json,
                updated_at = excluded.updated_at",
            params![session_id, user_skill, ctx.current_topic_index as i64, state_json, now],
        ).map_err(|e| CoreError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn load_session(&self, session_id: Uuid) -> CoreResult<Option<StateMachineContext>> {
        let sid_str = session_id.to_string();
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT state_json FROM sessions WHERE session_id = ?1")
            .map_err(|e| CoreError::StorageError(e.to_string()))?;

        let mut rows = stmt.query(params![sid_str])
            .map_err(|e| CoreError::StorageError(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| CoreError::StorageError(e.to_string()))? {
            let json_str: String = row.get(0).map_err(|e| CoreError::StorageError(e.to_string()))?;
            let ctx: StateMachineContext = serde_json::from_str(&json_str)
                .map_err(|e| CoreError::Serialization(e.to_string()))?;
            Ok(Some(ctx))
        } else {
            Ok(None)
        }
    }
}
