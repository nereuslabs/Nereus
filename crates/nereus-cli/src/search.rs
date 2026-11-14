//! Search command implementation querying SQLite FTS5 index.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use anyhow::Result;
use nereus_storage::SqliteStorage;
use std::sync::Arc;

/// Executes the `nereus search <query>` command.
pub async fn search_command(storage: &Arc<SqliteStorage>, query: &str, limit: usize) -> Result<()> {
    let results = storage.search_assessments_fts(query, limit).await?;

    if results.is_empty() {
        println!("🔍 No assessments found matching query: \"{query}\"");
        return Ok(());
    }

    println!(
        "🔍 Found {} matching assessment(s) for \"{}\":\n",
        results.len(),
        query
    );

    let headers = [
        "Session ID",
        "Topic",
        "Verdict",
        "Score",
        "Timestamp",
        "Feedback Snippet",
    ];

    let rows: Vec<Vec<String>> = results
        .iter()
        .map(|r| {
            let snippet = if r.feedback.len() > 60 {
                format!("{}...", &r.feedback[..57])
            } else {
                r.feedback.clone()
            };
            let ts = if r.timestamp.len() >= 19 {
                r.timestamp[..19].replace('T', " ")
            } else {
                r.timestamp.clone()
            };
            vec![
                r.session_id.to_string()[..8].to_string(),
                r.topic_id.clone(),
                r.verdict.to_uppercase(),
                format!("{:.1}%", r.score),
                ts,
                snippet,
            ]
        })
        .collect();

    println!("{}", crate::format::format_ascii_table(&headers, &rows));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nereus_core::models::UserProfile;
    use nereus_core::state::StateMachineContext;
    use nereus_storage::AssessmentRecord;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_search_command_empty_and_matching() {
        let storage = Arc::new(SqliteStorage::in_memory().unwrap());
        let session_id = Uuid::new_v4();
        let mut ctx = StateMachineContext::new(UserProfile::default(), false);
        ctx.session_id = session_id;
        storage.save_session_atomic(&ctx, None).await.unwrap();

        let a = AssessmentRecord::new(
            session_id,
            "ownership-topic",
            90.0,
            "pass",
            1,
            "Mastered move semantics and borrow checker rules cleanly.",
        );
        storage.record_assessment(&a).await.unwrap();

        let res = search_command(&storage, "semantics", 5).await;
        assert!(res.is_ok());

        let empty_res = search_command(&storage, "nonexistent", 5).await;
        assert!(empty_res.is_ok());
    }
}
