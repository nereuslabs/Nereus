//! Formatted ASCII table generator and presentation helpers for sessions and assessment history.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use nereus_storage::models::{AssessmentRecord, SessionRecord};

/// Generic ASCII table generator with dynamic column sizing and uniform borders.
pub fn format_ascii_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if headers.is_empty() {
        return String::new();
    }

    let num_cols = headers.len();
    let mut col_widths = vec![0; num_cols];

    for (i, header) in headers.iter().enumerate() {
        col_widths[i] = header.len();
    }

    for row in rows {
        for (i, cell) in row.iter().enumerate().take(num_cols) {
            col_widths[i] = col_widths[i].max(cell.len());
        }
    }

    let mut out = String::new();

    // Top border: +---+---+
    let separator = make_separator(&col_widths);
    out.push_str(&separator);
    out.push('\n');

    // Header row
    out.push_str(&make_row(headers, &col_widths));
    out.push('\n');

    // Header separator
    out.push_str(&separator);
    out.push('\n');

    // Data rows
    for row in rows {
        let cells: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
        out.push_str(&make_row(&cells, &col_widths));
        out.push('\n');
    }

    // Bottom border
    out.push_str(&separator);

    out
}

fn make_separator(col_widths: &[usize]) -> String {
    let mut s = String::from("+");
    for &w in col_widths {
        s.push_str(&"-".repeat(w + 2));
        s.push('+');
    }
    s
}

fn make_row(cells: &[&str], col_widths: &[usize]) -> String {
    let mut s = String::from("|");
    for (i, &w) in col_widths.iter().enumerate() {
        let cell = cells.get(i).copied().unwrap_or("");
        s.push(' ');
        s.push_str(cell);
        if cell.len() < w {
            s.push_str(&" ".repeat(w - cell.len()));
        }
        s.push_str(" |");
    }
    s
}

/// Formats a list of stored sessions into an ASCII table with columns:
/// Session ID, Skill, Current Topic, Last Updated.
pub fn format_sessions_table(records: &[SessionRecord]) -> String {
    if records.is_empty() {
        return "No stored sessions found in database.".to_string();
    }

    let headers = ["Session ID", "Skill", "Current Topic", "Last Updated"];
    let rows: Vec<Vec<String>> = records
        .iter()
        .map(|r| {
            let topic = resolve_current_topic(r);
            vec![
                r.session_id.to_string(),
                r.user_skill.clone(),
                topic,
                r.updated_at.clone(),
            ]
        })
        .collect();

    format_ascii_table(&headers, &rows)
}

/// Formats chronological assessment history into an ASCII table with columns:
/// Topic, Score, Verdict, Timestamp, Socratic Feedback Summary.
pub fn format_history_table(history: &[AssessmentRecord]) -> String {
    if history.is_empty() {
        return "No assessment history records found.".to_string();
    }

    let headers = [
        "Topic",
        "Score",
        "Verdict",
        "Timestamp",
        "Socratic Feedback Summary",
    ];
    let rows: Vec<Vec<String>> = history
        .iter()
        .map(|r| {
            vec![
                r.topic_id.clone(),
                format!("{:.1}", r.score),
                r.verdict.to_uppercase(),
                r.timestamp.clone(),
                summarize_feedback(&r.feedback, 50),
            ]
        })
        .collect();

    format_ascii_table(&headers, &rows)
}

/// Resolves a human-readable topic title or status from a `SessionRecord`.
pub fn resolve_current_topic(record: &SessionRecord) -> String {
    match record.context() {
        Ok(ctx) => {
            if ctx.is_completed() {
                "Completed".to_string()
            } else if let Some(topic) = ctx.current_topic() {
                topic.title.clone()
            } else {
                format!("Topic #{}", record.current_topic_index + 1)
            }
        }
        Err(_) => format!("Topic #{}", record.current_topic_index + 1),
    }
}

/// Summarizes Socratic feedback to a single-line string capped at `max_len` characters.
pub fn summarize_feedback(feedback: &str, max_len: usize) -> String {
    let single_line = feedback.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() > max_len {
        let truncated: String = single_line
            .chars()
            .take(max_len.saturating_sub(3))
            .collect();
        format!("{}...", truncated.trim_end())
    } else {
        single_line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nereus_core::models::{Roadmap, RoadmapTopic, UserLevel, UserProfile};
    use nereus_core::state::{LearningState, StateMachineContext};
    use uuid::Uuid;

    #[test]
    fn test_format_ascii_table_empty() {
        assert_eq!(format_ascii_table(&[], &[]), "");
    }

    #[test]
    fn test_format_ascii_table_layout() {
        let headers = ["Col A", "Column B"];
        let rows = vec![
            vec!["hello".to_string(), "world".to_string()],
            vec!["foo".to_string(), "bar baz".to_string()],
        ];

        let table = format_ascii_table(&headers, &rows);
        assert!(table.contains("+-------+----------+"));
        assert!(table.contains("| Col A | Column B |"));
        assert!(table.contains("| hello | world    |"));
        assert!(table.contains("| foo   | bar baz  |"));
    }

    #[test]
    fn test_summarize_feedback() {
        assert_eq!(summarize_feedback("", 20), "");
        assert_eq!(summarize_feedback("Short", 20), "Short");
        assert_eq!(
            summarize_feedback("Notice compiler error E0382: ownership moved.", 25),
            "Notice compiler error..."
        );
        assert_eq!(
            summarize_feedback("line 1\n   line 2\n\nline 3", 50),
            "line 1 line 2 line 3"
        );
    }

    #[test]
    fn test_format_sessions_table_empty() {
        let out = format_sessions_table(&[]);
        assert_eq!(out, "No stored sessions found in database.");
    }

    #[test]
    fn test_format_sessions_table_with_records() {
        let session_id = Uuid::new_v4();
        let profile = UserProfile {
            skill: "Rust".to_string(),
            current_level: UserLevel::Intermediate,
            target_level: UserLevel::Advanced,
            hours_per_day: 2.0,
            deadline_days: 14,
            goal: "Master async".to_string(),
        };
        let mut ctx = StateMachineContext::new(profile, false);
        ctx.roadmap = Roadmap {
            topics: vec![RoadmapTopic {
                id: "topic-1".to_string(),
                title: "Memory Safety".to_string(),
                description: "Ownership and move semantics".to_string(),
                difficulty: 0.5,
                prerequisites: vec![],
                estimated_hours: 2.0,
            }],
        };
        ctx.current_topic_index = 0;
        let state_json = serde_json::to_string(&ctx).unwrap();

        let record = SessionRecord::new(
            session_id,
            "Rust",
            0,
            state_json,
            "2026-11-09T12:00:00+00:00",
        );

        let table = format_sessions_table(&[record]);
        assert!(table.contains("Session ID"));
        assert!(table.contains("Skill"));
        assert!(table.contains("Current Topic"));
        assert!(table.contains("Last Updated"));
        assert!(table.contains(&session_id.to_string()));
        assert!(table.contains("Rust"));
        assert!(table.contains("Memory Safety"));
        assert!(table.contains("2026-11-09T12:00:00+00:00"));
    }

    #[test]
    fn test_resolve_current_topic_states() {
        let session_id = Uuid::new_v4();
        let profile = UserProfile::default();
        let mut ctx = StateMachineContext::new(profile, false);
        ctx.state = LearningState::Completed;
        let state_json = serde_json::to_string(&ctx).unwrap();

        let record = SessionRecord::new(
            session_id,
            "Rust",
            0,
            state_json,
            "2026-11-09T12:00:00+00:00",
        );
        assert_eq!(resolve_current_topic(&record), "Completed");

        let corrupt_record = SessionRecord::new(
            session_id,
            "Rust",
            2,
            "invalid-json",
            "2026-11-09T12:00:00+00:00",
        );
        assert_eq!(resolve_current_topic(&corrupt_record), "Topic #3");
    }

    #[test]
    fn test_format_history_table() {
        assert_eq!(
            format_history_table(&[]),
            "No assessment history records found."
        );

        let sid = Uuid::new_v4();
        let record = AssessmentRecord::with_id(
            "rec-1",
            sid,
            "topic-1",
            92.5,
            "pass",
            1,
            "Sandbox assertions passed cleanly.",
            "2026-11-09T12:05:00+00:00",
        );

        let table = format_history_table(&[record]);
        assert!(table.contains("Topic"));
        assert!(table.contains("Score"));
        assert!(table.contains("Verdict"));
        assert!(table.contains("Timestamp"));
        assert!(table.contains("Socratic Feedback Summary"));
        assert!(table.contains("topic-1"));
        assert!(table.contains("92.5"));
        assert!(table.contains("PASS"));
        assert!(table.contains("Sandbox assertions passed cleanly."));
    }
}
