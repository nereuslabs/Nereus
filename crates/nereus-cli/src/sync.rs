//! Git-backed session synchronization command.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use anyhow::{Context, Result};
use nereus_storage::SqliteStorage;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

/// Resolves the default journal path `~/.nereus/journal` or user-supplied path.
pub fn resolve_journal_path(custom_path: Option<&str>) -> PathBuf {
    if let Some(p) = custom_path {
        crate::db::expand_tilde(p)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".nereus").join("journal")
    } else {
        PathBuf::from(".nereus_journal")
    }
}

/// Executes the `nereus sync` command to persist Markdown learning journals to Git.
pub async fn sync_command(
    storage: &Arc<SqliteStorage>,
    custom_path: Option<&str>,
    remote: Option<&str>,
    commit_msg: &str,
    dry_run: bool,
) -> Result<()> {
    let journal_dir = resolve_journal_path(custom_path);
    let sessions = storage.list_sessions().await?;

    println!(
        "🔄 Git Session Sync: Preparing journal at {}",
        journal_dir.display()
    );

    if dry_run {
        println!(
            "🧪 Dry-run mode enabled: No filesystem changes or Git operations will be performed."
        );
        println!(
            "📊 Discovered {} session(s) eligible for synchronization.",
            sessions.len()
        );
        for s in &sessions {
            println!(
                "   - Session {} (skill: {}, updated: {})",
                s.session_id, s.user_skill, s.updated_at
            );
        }
        return Ok(());
    }

    fs::create_dir_all(&journal_dir).with_context(|| {
        format!(
            "Failed to create journal directory: {}",
            journal_dir.display()
        )
    })?;

    let mut synced_count = 0;
    for s in &sessions {
        if let Ok(ctx) = s.context() {
            let history = storage
                .get_assessment_history(s.session_id)
                .await
                .unwrap_or_default();
            let md = crate::export::render_session_markdown(&ctx, &history);
            let file_path = journal_dir.join(format!("session_{}.md", s.session_id));
            fs::write(&file_path, md).with_context(|| {
                format!("Failed to write session journal: {}", file_path.display())
            })?;
            synced_count += 1;
        }
    }

    // Write index README.md
    let index_path = journal_dir.join("README.md");
    let index_content = format!(
        "# Nereus Learning Journal\n\nAutomatically synchronized at {}\n\nTotal Sessions: {}\n",
        chrono::Utc::now().to_rfc3339(),
        synced_count
    );
    let _ = fs::write(&index_path, index_content);

    // Initialize git repository if not already initialized
    let git_dir = journal_dir.join(".git");
    if !git_dir.exists() {
        let _ = Command::new("git")
            .arg("init")
            .current_dir(&journal_dir)
            .output();
    }

    // Git add .
    let add_status = Command::new("git")
        .args(["add", "."])
        .current_dir(&journal_dir)
        .status();

    if let Ok(status) = add_status {
        if status.success() {
            // Commit
            let _ = Command::new("git")
                .args(["commit", "-m", commit_msg])
                .current_dir(&journal_dir)
                .output();

            // Push if remote configured
            if let Some(remote_url) = remote {
                println!("🚀 Pushing journal to remote: {remote_url}");
                let _ = Command::new("git")
                    .args(["remote", "add", "origin", remote_url])
                    .current_dir(&journal_dir)
                    .output();
                let _ = Command::new("git")
                    .args(["push", "-u", "origin", "main"])
                    .current_dir(&journal_dir)
                    .output();
            }
        }
    }

    println!(
        "✅ Synchronized {} session journal(s) to {}",
        synced_count,
        journal_dir.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nereus_core::models::UserProfile;
    use nereus_core::state::StateMachineContext;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_sync_command_dry_run_and_execution() {
        let storage = Arc::new(SqliteStorage::in_memory().unwrap());
        let session_id = Uuid::new_v4();
        let mut ctx = StateMachineContext::new(UserProfile::default(), false);
        ctx.session_id = session_id;
        storage.save_session_atomic(&ctx, None).await.unwrap();

        let temp_dir = std::env::temp_dir().join(format!("nereus-test-sync-{}", Uuid::new_v4()));
        let journal_path = temp_dir.to_str().unwrap();

        // 1. Dry run
        let dry_res = sync_command(&storage, Some(journal_path), None, "test", true).await;
        assert!(dry_res.is_ok());

        // 2. Real execution
        let real_res = sync_command(&storage, Some(journal_path), None, "test sync", false).await;
        assert!(real_res.is_ok());

        // Verify session markdown file written
        let expected_file = temp_dir.join(format!("session_{session_id}.md"));
        assert!(expected_file.exists());
        let expected_readme = temp_dir.join("README.md");
        assert!(expected_readme.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
