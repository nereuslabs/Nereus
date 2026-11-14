//! Session explorer command handlers and lifecycle execution.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crate::format::{format_history_table, format_sessions_table};
use anyhow::Result;
use nereus_storage::SqliteStorage;
use uuid::Uuid;

/// Executes `nereus sessions list` by querying stored sessions and printing an ASCII table.
pub async fn list_sessions_command(storage: &SqliteStorage) -> Result<()> {
    let sessions = storage.list_sessions().await?;
    if sessions.is_empty() {
        println!("No stored sessions found in database.");
    } else {
        println!("{}", format_sessions_table(&sessions));
    }
    Ok(())
}

/// Executes `nereus sessions history <UUID>` by querying assessment history and printing an ASCII table.
pub async fn history_command(storage: &SqliteStorage, session_id: Uuid) -> Result<()> {
    let history = storage.get_assessment_history(session_id).await?;
    if history.is_empty() {
        let session_opt =
            nereus_core::traits::StorageBackend::load_session(storage, session_id).await?;
        if session_opt.is_some() {
            println!("No assessment history found for session {}.", session_id);
        } else {
            println!("Session '{}' not found in database.", session_id);
        }
    } else {
        println!("Assessment History for Session: {}\n", session_id);
        println!("{}", format_history_table(&history));
    }
    Ok(())
}

/// Executes `nereus sessions delete <UUID>` by deleting the session and cascading to assessment history.
pub async fn delete_session_command(storage: &SqliteStorage, session_id: Uuid) -> Result<()> {
    let deleted = storage.delete_session(session_id).await?;
    if deleted {
        println!(
            "Successfully deleted session {} and its assessment history.",
            session_id
        );
    } else {
        println!("Session '{}' not found in database.", session_id);
    }
    Ok(())
}
