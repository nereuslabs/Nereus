//! Cockpit UI state and view rendering.

use crate::layout::calculate_cockpit_layout;
use nereus_core::state::{LearningState, StateMachineContext};
use ratatui::backend::Backend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct CockpitApp {
    pub should_quit: bool,
}

impl Default for CockpitApp {
    fn default() -> Self {
        Self { should_quit: false }
    }
}

impl CockpitApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn draw(&self, frame: &mut Frame, ctx: &StateMachineContext) {
        let area = frame.size();
        let panes = calculate_cockpit_layout(area);

        // 1. Roadmap Pane
        let mut roadmap_items = Vec::new();
        for (i, topic) in ctx.roadmap.topics.iter().enumerate() {
            let prefix = if i < ctx.current_topic_index {
                Span::styled("[✔] ", Style::default().fg(Color::Green))
            } else if i == ctx.current_topic_index {
                Span::styled("[●] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
            };

            let title = Span::raw(format!("{}. {}", i + 1, topic.title));
            roadmap_items.push(Line::from(vec![prefix, title]));
        }

        if roadmap_items.is_empty() {
            roadmap_items.push(Line::from(vec![Span::raw("Roadmap compiling...")]));
        }

        let roadmap_block = Block::default()
            .title(" 🗺️ Roadmap ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        frame.render_widget(Paragraph::new(roadmap_items).block(roadmap_block), panes.roadmap);

        // 2. Task Pane
        let task_text = match &ctx.state {
            LearningState::Tutoring { material, task, .. } => format!("{material}\n\n---\n🎯 Task:\n{task}"),
            LearningState::Examining { task, .. } => format!("🎯 Active Task:\n{task}\n\nWaiting for student code submission..."),
            LearningState::Diagnostic { .. } => "🔍 Baseline diagnostic in progress. Answer questions in the terminal.".to_string(),
            LearningState::Coaching { .. } => "🧭 Synthesizing personalized roadmap from identified weak areas...".to_string(),
            LearningState::Completed => "🎉 All roadmap milestones completed! You have mastered this curriculum.".to_string(),
        };

        let task_block = Block::default()
            .title(" 🧪 Challenge & Theory ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        frame.render_widget(Paragraph::new(task_text).block(task_block), panes.task);

        // 3. Socratic Guidance Pane
        let guidance_text = match &ctx.state {
            LearningState::Examining { assessment: Some(a), .. } => format!("> 🗼 Lighthouse Beacon:\n{}", a.feedback),
            LearningState::Tutoring { retry_count, .. } if *retry_count > 0 => {
                "> 💡 Socratic Hint:\nReflect on the compiler error. Does the lifetime outlive the caller scope?".to_string()
            }
            _ => "> 🗼 Lighthouse Beacon:\nRead the theory carefully before starting the challenge.".to_string(),
        };

        let socratic_block = Block::default()
            .title(" 💡 Socratic Guidance ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        frame.render_widget(Paragraph::new(guidance_text).block(socratic_block), panes.socratic_guidance);

        // 4. Editor Pane
        let editor_text = match &ctx.state {
            LearningState::Examining { submission: Some(sub), .. } => sub.clone(),
            _ => "// Type or paste your solution here\npub async fn solve() -> Result<(), Box<dyn std::error::Error>> {\n    // Implementation\n    Ok(())\n}".to_string(),
        };

        let editor_block = Block::default()
            .title(" 📝 Code Submission ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));
        frame.render_widget(Paragraph::new(editor_text).block(editor_block), panes.code_editor);
    }
}
