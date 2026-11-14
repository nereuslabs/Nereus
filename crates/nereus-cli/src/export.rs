//! Session export and ASCII terminal replay generator.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.
//!
//! Exports stored learning session milestones, compiler diagnostics, and interactive histories
//! into standalone Markdown reports, Asciinema v2 `.cast` terminal replays, and SVG badges.

use anyhow::Result;
use nereus_core::state::StateMachineContext;
use nereus_core::traits::StorageBackend;
use nereus_storage::{AssessmentRecord, SqliteStorage};
use serde_json::json;
use uuid::Uuid;

/// Generates a comprehensive GitHub Flavored Markdown summary report of a learning session.
pub fn render_session_markdown(ctx: &StateMachineContext, history: &[AssessmentRecord]) -> String {
    let mut md = String::new();
    md.push_str("# 🌊 Nereus Learning Session Report\n\n");
    md.push_str(&format!("**Session ID**: `{}`  \n", ctx.session_id));
    md.push_str(&format!("**Target Skill**: `{}`  \n", ctx.profile.skill));
    md.push_str(&format!(
        "**Learner Baseline**: `{:?}` ➔ Target: `{:?}`  \n",
        ctx.profile.current_level, ctx.profile.target_level
    ));
    md.push_str(&format!("**Learning Goal**: {}  \n", ctx.profile.goal));
    md.push_str(&format!("**Current State**: `{}`  \n", ctx.state.name()));

    let total_topics = ctx.roadmap.topics.len();
    let current_index = ctx.current_topic_index;
    let progress_pct = if total_topics > 0 {
        if ctx.is_completed() {
            100.0
        } else {
            (current_index as f32 / total_topics as f32) * 100.0
        }
    } else {
        0.0
    };
    md.push_str(&format!(
        "**Curriculum Progress**: `{:.1}%` ({}/{} Topics)\n\n",
        progress_pct, current_index, total_topics
    ));

    md.push_str("## 📚 Curriculum Roadmap & Topic Status\n\n");
    md.push_str("| # | Topic ID | Title | Difficulty | Est. Hours | Status |\n");
    md.push_str("| :-: | :--- | :--- | :-: | :-: | :--- |\n");

    for (idx, topic) in ctx.roadmap.topics.iter().enumerate() {
        let status = if idx < current_index || ctx.is_completed() {
            "✅ Mastered"
        } else if idx == current_index {
            "⚡ In Progress"
        } else {
            "⏳ Pending"
        };
        md.push_str(&format!(
            "| {} | `{}` | {} | {:.1} | {:.1}h | {} |\n",
            idx + 1,
            topic.id,
            topic.title,
            topic.difficulty,
            topic.estimated_hours,
            status
        ));
    }

    md.push_str("\n## 🔍 Socratic Assessment History\n\n");
    if history.is_empty() {
        md.push_str("_No formal assessments recorded yet for this session._\n");
    } else {
        md.push_str("| Timestamp | Topic ID | Score | Verdict | Attempts | Feedback Pointer |\n");
        md.push_str("| :--- | :--- | :-: | :-: | :-: | :--- |\n");
        for record in history {
            let verdict_badge = if record.verdict.to_lowercase() == "pass" {
                "🟢 PASS"
            } else {
                "🟡 RETRY"
            };
            let short_feedback = if record.feedback.len() > 64 {
                format!("{}...", &record.feedback[..61])
            } else {
                record.feedback.clone()
            };
            md.push_str(&format!(
                "| `{}` | `{}` | {:.1} | {} | {} | {} |\n",
                record.timestamp,
                record.topic_id,
                record.score,
                verdict_badge,
                record.attempts,
                short_feedback.replace('|', "\\|").replace('\n', " ")
            ));
        }
    }

    md
}

/// Generates an Asciinema v2 (`.cast`) JSON event stream replayable in terminals or web players.
pub fn render_session_cast(ctx: &StateMachineContext, history: &[AssessmentRecord]) -> String {
    let mut lines = Vec::new();

    // Line 1: Asciinema v2 header
    let header = json!({
        "version": 2,
        "width": 100,
        "height": 32,
        "timestamp": chrono::Utc::now().timestamp(),
        "title": format!("Nereus 2.0 Socratic Session: {}", ctx.profile.skill),
        "env": {
            "SHELL": "/bin/zsh",
            "TERM": "xterm-256color"
        }
    });
    lines.push(header.to_string());

    let mut t = 0.0_f64;

    // Helper macro/closure to add output event
    let mut add_event = |time_delta: f64, text: &str| {
        t += time_delta;
        let event = json!([t, "o", text]);
        lines.push(event.to_string());
    };

    // 1. Initial terminal clear and banner
    add_event(
        0.1,
        "\x1b[2J\x1b[H\x1b[36m🌊 NEREUS 2.0 TERMINAL AI MENTOR\x1b[0m\r\n",
    );
    add_event(
        0.2,
        "======================================================================\r\n",
    );
    add_event(
        0.2,
        &format!("[*] Loading Session: \x1b[33m{}\x1b[0m\r\n", ctx.session_id),
    );
    add_event(
        0.1,
        &format!(
            "[*] Target Skill: \x1b[32m{}\x1b[0m (Goal: {})\r\n\r\n",
            ctx.profile.skill, ctx.profile.goal
        ),
    );

    // 2. Initial state
    add_event(
        0.3,
        &format!(
            "[FSM] State initialized: \x1b[1m{}\x1b[0m\r\n",
            ctx.state.name()
        ),
    );

    // 3. Replay assessments
    for (i, record) in history.iter().enumerate() {
        add_event(
            0.4,
            &format!(
                "\r\n--- Step {}: Evaluation for Topic «\x1b[36m{}\x1b[0m» ---\r\n",
                i + 1,
                record.topic_id
            ),
        );
        let verdict_color = if record.verdict.to_lowercase() == "pass" {
            "\x1b[32mPASS\x1b[0m"
        } else {
            "\x1b[33mRETRY\x1b[0m"
        };
        add_event(
            0.3,
            &format!(
                "    Verdict: {} | Score: {:.1} (Attempt {})\r\n",
                verdict_color, record.score, record.attempts
            ),
        );
        add_event(
            0.2,
            &format!(
                "    Feedback: {}\r\n",
                record.feedback.replace('\n', "\r\n    ")
            ),
        );
    }

    // 4. Final summary
    add_event(
        0.5,
        "\r\n======================================================================\r\n",
    );
    let status_str = if ctx.is_completed() {
        "\x1b[32m🎉 COMPLETE - All curriculum topics mastered!\x1b[0m"
    } else {
        "\x1b[36m⚡ ACTIVE - Ready to resume session.\x1b[0m"
    };
    add_event(0.2, &format!("Status: {}\r\n", status_str));

    lines.join("\n")
}

/// Generates an interactive SVG terminal frame displaying session status and metrics.
pub fn render_session_svg(ctx: &StateMachineContext, history: &[AssessmentRecord]) -> String {
    let total_topics = ctx.roadmap.topics.len();
    let current_index = ctx.current_topic_index;
    let progress_pct = if total_topics > 0 {
        if ctx.is_completed() {
            100.0
        } else {
            (current_index as f32 / total_topics as f32) * 100.0
        }
    } else {
        0.0
    };

    let pass_count = history
        .iter()
        .filter(|r| r.verdict.to_lowercase() == "pass")
        .count();

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 360" width="100%" height="100%">
  <defs>
    <linearGradient id="bgGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0f172a" />
      <stop offset="100%" stop-color="#1e293b" />
    </linearGradient>
    <linearGradient id="accentGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#38bdf8" />
      <stop offset="100%" stop-color="#818cf8" />
    </linearGradient>
  </defs>

  <!-- Terminal Window Background -->
  <rect width="760" height="360" rx="12" fill="url(#bgGrad)" stroke="#334155" stroke-width="1.5" />

  <!-- Window Header Bar -->
  <path d="M 0 12 C 0 5.37 5.37 0 12 0 L 748 0 C 754.63 0 760 5.37 760 12 L 760 40 L 0 40 Z" fill="#1e293b" />
  <circle cx="24" cy="20" r="6" fill="#ef4444" />
  <circle cx="44" cy="20" r="6" fill="#f59e0b" />
  <circle cx="64" cy="20" r="6" fill="#10b981" />
  <text x="380" y="25" fill="#94a3b8" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" font-size="13" font-weight="600" text-anchor="middle">
    nereus 2.0 — session-{}
  </text>

  <!-- Session Content -->
  <text x="32" y="75" fill="#38bdf8" font-family="'JetBrains Mono', monospace" font-size="18" font-weight="bold">
    🌊 Nereus Socratic Mentor Cockpit
  </text>
  <text x="32" y="105" fill="#e2e8f0" font-family="-apple-system, BlinkMacSystemFont, sans-serif" font-size="14">
    Skill: <tspan fill="#38bdf8" font-weight="bold">{}</tspan> • State: <tspan fill="#f59e0b" font-weight="bold">{}</tspan>
  </text>

  <!-- Progress Bar Container -->
  <rect x="32" y="130" width="696" height="12" rx="6" fill="#334155" />
  <rect x="32" y="130" width="{}" height="12" rx="6" fill="url(#accentGrad)" />

  <!-- Metric Badges -->
  <rect x="32" y="165" width="216" height="70" rx="8" fill="#1e293b" stroke="#334155" stroke-width="1" />
  <text x="48" y="192" fill="#94a3b8" font-family="sans-serif" font-size="12">CURRICULUM MASTERY</text>
  <text x="48" y="222" fill="#38bdf8" font-family="'JetBrains Mono', monospace" font-size="22" font-weight="bold">{:.1}%</text>

  <rect x="272" y="165" width="216" height="70" rx="8" fill="#1e293b" stroke="#334155" stroke-width="1" />
  <text x="288" y="192" fill="#94a3b8" font-family="sans-serif" font-size="12">TOPICS COMPLETED</text>
  <text x="288" y="222" fill="#10b981" font-family="'JetBrains Mono', monospace" font-size="22" font-weight="bold">{}/{}</text>

  <rect x="512" y="165" width="216" height="70" rx="8" fill="#1e293b" stroke="#334155" stroke-width="1" />
  <text x="528" y="192" fill="#94a3b8" font-family="sans-serif" font-size="12">ASSESSMENTS PASSED</text>
  <text x="528" y="222" fill="#818cf8" font-family="'JetBrains Mono', monospace" font-size="22" font-weight="bold">{}</text>

  <!-- Terminal Footer Details -->
  <text x="32" y="280" fill="#64748b" font-family="'JetBrains Mono', monospace" font-size="12">
    Goal: {}
  </text>
  <text x="32" y="305" fill="#64748b" font-family="'JetBrains Mono', monospace" font-size="12">
    Last Activity: {} • Total Assessments Logged: {}
  </text>
  <text x="32" y="335" fill="#475569" font-family="'JetBrains Mono', monospace" font-size="11">
    Nereus 2.0 Autonomous Pedagogical System • Zero Token Telemetry
  </text>
</svg>"##,
        &ctx.session_id.to_string()[..8],
        ctx.profile.skill,
        ctx.state.name(),
        (696.0 * (progress_pct / 100.0)).max(12.0),
        progress_pct,
        current_index,
        total_topics,
        pass_count,
        if ctx.profile.goal.len() > 64 {
            format!("{}...", &ctx.profile.goal[..61])
        } else {
            ctx.profile.goal.clone()
        },
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"),
        history.len()
    )
}

/// Executes the `nereus session export` CLI command.
pub async fn export_session_command(
    storage: &SqliteStorage,
    session_id: Uuid,
    format: &str,
    output_path: Option<&str>,
) -> Result<()> {
    let ctx = match storage.load_session(session_id).await? {
        Some(c) => c,
        None => anyhow::bail!("Session '{}' not found in database", session_id),
    };

    let history = storage.get_assessment_history(session_id).await?;

    let rendered = match format.to_lowercase().as_str() {
        "markdown" | "md" => render_session_markdown(&ctx, &history),
        "cast" | "asciinema" => render_session_cast(&ctx, &history),
        "svg" | "badge" => render_session_svg(&ctx, &history),
        other => anyhow::bail!(
            "Unsupported export format '{}'. Supported: markdown, cast, svg",
            other
        ),
    };

    if let Some(path) = output_path {
        std::fs::write(path, &rendered)?;
        println!(
            "✨ Exported session {} [{}] to: {}",
            session_id, format, path
        );
    } else {
        println!("{}", rendered);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nereus_core::models::{Roadmap, RoadmapTopic, UserLevel, UserProfile};

    fn make_test_context() -> StateMachineContext {
        let profile = UserProfile {
            skill: "Rust Concurrency".to_string(),
            current_level: UserLevel::Intermediate,
            target_level: UserLevel::Advanced,
            hours_per_day: 2.0,
            deadline_days: 14,
            goal: "Master async channels and locks".to_string(),
        };

        let topic1 = RoadmapTopic {
            id: "async-channels".to_string(),
            title: "Asynchronous Channels".to_string(),
            description: "MPSC and Broadcast primitives".to_string(),
            difficulty: 0.6,
            prerequisites: vec![],
            estimated_hours: 2.0,
        };

        let mut ctx = StateMachineContext::new(profile, false);
        ctx.roadmap = Roadmap {
            topics: vec![topic1],
        };
        ctx
    }

    #[test]
    fn test_render_session_markdown() {
        let ctx = make_test_context();
        let history = vec![AssessmentRecord::new(
            ctx.session_id,
            "async-channels",
            92.0,
            "pass",
            1,
            "Clean channel implementation",
        )];

        let md = render_session_markdown(&ctx, &history);
        assert!(md.contains("# 🌊 Nereus Learning Session Report"));
        assert!(md.contains("Rust Concurrency"));
        assert!(md.contains("async-channels"));
        assert!(md.contains("🟢 PASS"));
    }

    #[test]
    fn test_render_session_cast() {
        let ctx = make_test_context();
        let history = vec![AssessmentRecord::new(
            ctx.session_id,
            "async-channels",
            92.0,
            "pass",
            1,
            "Clean channel implementation",
        )];

        let cast = render_session_cast(&ctx, &history);
        let lines: Vec<&str> = cast.lines().collect();
        assert!(lines.len() >= 3);

        // First line must be valid Asciinema header JSON
        let header_json: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header_json["version"], 2);
        assert_eq!(header_json["width"], 100);
    }

    #[test]
    fn test_render_session_svg() {
        let ctx = make_test_context();
        let history = vec![];

        let svg = render_session_svg(&ctx, &history);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("Rust Concurrency"));
        assert!(svg.contains("CURRICULUM MASTERY"));
    }
}
