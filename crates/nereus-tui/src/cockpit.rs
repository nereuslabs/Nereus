//! Cockpit UI state, interactive event loop, and view rendering.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crate::events::{AppAction, AppEvent, EventHandler};
use crate::layout::calculate_cockpit_layout;
use crate::widgets::{
    centered_rect, DiagnosticQuizWidget, EditorState, EditorWidget, HelpWidget, LighthouseWidget,
    QuizState, RoadmapState, RoadmapWidget, StatusBarWidget, TheoryViewerWidget,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nereus_core::state::{LearningState, StateMachineContext};
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// The four interactive panes in the Nereus Cockpit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    /// Curriculum topics and mastery progress
    #[default]
    Roadmap,
    /// Challenge theory, concept notes, and prompt specification
    Task,
    /// Socratic hints, animated lighthouse beacon, and compiler feedback
    Guidance,
    /// Student code submission editor buffer
    Editor,
}

impl FocusedPane {
    /// Advances focus clockwise across cockpit panes.
    pub fn next(&self) -> Self {
        match self {
            FocusedPane::Roadmap => FocusedPane::Task,
            FocusedPane::Task => FocusedPane::Guidance,
            FocusedPane::Guidance => FocusedPane::Editor,
            FocusedPane::Editor => FocusedPane::Roadmap,
        }
    }

    /// Cycles focus counter-clockwise across cockpit panes.
    pub fn prev(&self) -> Self {
        match self {
            FocusedPane::Roadmap => FocusedPane::Editor,
            FocusedPane::Task => FocusedPane::Roadmap,
            FocusedPane::Guidance => FocusedPane::Task,
            FocusedPane::Editor => FocusedPane::Guidance,
        }
    }

    /// Display title for the pane.
    pub fn title(&self) -> &'static str {
        match self {
            FocusedPane::Roadmap => "Roadmap",
            FocusedPane::Task => "Challenge & Theory",
            FocusedPane::Guidance => "Socratic Guidance",
            FocusedPane::Editor => "Code Submission",
        }
    }
}

/// Central application state for the Nereus Ratatui TUI cockpit.
pub struct CockpitApp {
    /// Currently focused interactive pane
    pub focused_pane: FocusedPane,
    /// Flag signaling graceful termination of the event loop
    pub should_quit: bool,
    /// Monotonically increasing tick counter driving animations
    pub tick_count: u64,
    /// Vertical scroll offset for the Challenge & Theory pane
    pub task_scroll: usize,
    /// Vertical scroll offset for the Socratic Guidance pane
    pub guidance_scroll: usize,
    /// State tracker for roadmap selection and scrolling
    pub roadmap_state: RoadmapState,
    /// State tracker for code submission editor buffer, cursor, and scroll
    pub editor_state: EditorState,
    /// State tracker for baseline diagnostic quiz selection cursor
    pub quiz_state: QuizState,
    /// Flag indicating whether the Help modal overlay is visible
    pub show_help: bool,
    /// Most recently submitted code snippet
    pub last_submission: Option<String>,
    /// Ephemeral status bar message
    pub status_message: Option<String>,
}

impl Default for CockpitApp {
    fn default() -> Self {
        Self {
            focused_pane: FocusedPane::Roadmap,
            should_quit: false,
            tick_count: 0,
            task_scroll: 0,
            guidance_scroll: 0,
            roadmap_state: RoadmapState::new(),
            editor_state: EditorState::default(),
            quiz_state: QuizState::new(),
            show_help: false,
            last_submission: None,
            status_message: None,
        }
    }
}

impl CockpitApp {
    /// Creates a fresh CockpitApp with default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes a CockpitApp preloaded with custom solution code.
    pub fn with_code(code: &str) -> Self {
        let mut app = Self::default();
        app.editor_state.set_code(code);
        app
    }

    /// Advances active focus to the next pane in sequence.
    pub fn cycle_focus(&mut self) {
        self.focused_pane = self.focused_pane.next();
        self.sync_focus_states();
    }

    /// Retreats active focus to the previous pane in sequence.
    pub fn cycle_focus_reverse(&mut self) {
        self.focused_pane = self.focused_pane.prev();
        self.sync_focus_states();
    }

    /// Directly sets focus to a specific pane.
    pub fn set_focus(&mut self, pane: FocusedPane) {
        self.focused_pane = pane;
        self.sync_focus_states();
    }

    /// Scrolls content upward in the currently focused pane.
    pub fn scroll_up(&mut self) {
        match self.focused_pane {
            FocusedPane::Roadmap => {
                self.roadmap_state.select_prev();
                self.roadmap_state.scroll_up(1);
            }
            FocusedPane::Task => {
                self.task_scroll = self.task_scroll.saturating_sub(1);
            }
            FocusedPane::Guidance => {
                self.guidance_scroll = self.guidance_scroll.saturating_sub(1);
            }
            FocusedPane::Editor => {
                self.editor_state.move_cursor_up();
                self.editor_state.scroll_up(1);
            }
        }
    }

    /// Scrolls content downward in the currently focused pane.
    pub fn scroll_down(&mut self) {
        match self.focused_pane {
            FocusedPane::Roadmap => {
                self.roadmap_state.select_next(100);
                self.roadmap_state.scroll_down(1, 100, 20);
            }
            FocusedPane::Task => {
                self.task_scroll = self.task_scroll.saturating_add(1);
            }
            FocusedPane::Guidance => {
                self.guidance_scroll = self.guidance_scroll.saturating_add(1);
            }
            FocusedPane::Editor => {
                self.editor_state.move_cursor_down();
                self.editor_state.scroll_down(1);
            }
        }
    }

    /// Submits the active editor code buffer.
    pub fn submit_code(&mut self) -> String {
        let code = self.editor_state.get_code();
        self.last_submission = Some(code.clone());
        self.status_message = Some("Code submitted for Socratic evaluation".to_string());
        code
    }

    /// Increments animation tick counter.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    /// Interactive event handler processing `crossterm::event::KeyEvent`.
    pub fn handle_key(&mut self, key: KeyEvent, ctx: &mut StateMachineContext) -> AppAction {
        // Global termination shortcuts: Ctrl+C
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return AppAction::Quit;
        }

        // When Help modal overlay is open, dismiss it on Esc, ?, or q
        if self.show_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.show_help = false;
            }
            return AppAction::None;
        }

        // Toggle help overlay with '?' when not typing inside the editor
        if key.code == KeyCode::Char('?') && self.focused_pane != FocusedPane::Editor {
            self.show_help = true;
            return AppAction::None;
        }

        // Tab focus cycling across panes
        if key.code == KeyCode::Tab {
            self.cycle_focus();
            return AppAction::FocusChanged(self.focused_pane);
        }

        if key.code == KeyCode::BackTab {
            self.cycle_focus_reverse();
            return AppAction::FocusChanged(self.focused_pane);
        }

        // In Diagnostic state, handle quiz question answers (1-4 or Up/Down + Enter)
        if let LearningState::Diagnostic {
            questions,
            current_index,
            answers,
        } = &mut ctx.state
        {
            if let Some(q) = questions.get(*current_index) {
                if let Some(chosen_idx) = self.quiz_state.handle_key(key, q.options.len()) {
                    let answer = q.options[chosen_idx].clone();
                    answers.push(answer.clone());
                    *current_index += 1;
                    self.quiz_state.selected_index = 0;
                    self.status_message =
                        Some(format!("Answered question {}: {}", *current_index, answer));
                    return AppAction::Submit(answer);
                }
                if matches!(
                    key.code,
                    KeyCode::Up | KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('k')
                ) {
                    return AppAction::None;
                }
            }
        }

        // Editor pane specific interactive character editing and cursor movements
        if self.focused_pane == FocusedPane::Editor {
            // Direct submission shortcut from editor
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Enter {
                let submission = self.submit_code();
                if let LearningState::Examining {
                    submission: sub, ..
                } = &mut ctx.state
                {
                    *sub = Some(submission.clone());
                }
                return AppAction::Submit(submission);
            }

            // Universal exit with Esc
            if key.code == KeyCode::Esc {
                self.should_quit = true;
                return AppAction::Quit;
            }

            match key.code {
                KeyCode::Enter => {
                    self.editor_state.insert_newline();
                    return AppAction::BufferModified;
                }
                KeyCode::Backspace => {
                    self.editor_state.delete_backspace();
                    return AppAction::BufferModified;
                }
                KeyCode::Delete => {
                    self.editor_state.delete_char();
                    return AppAction::BufferModified;
                }
                KeyCode::Left => {
                    self.editor_state.move_cursor_left();
                    return AppAction::None;
                }
                KeyCode::Right => {
                    self.editor_state.move_cursor_right();
                    return AppAction::None;
                }
                KeyCode::Up => {
                    self.editor_state.move_cursor_up();
                    return AppAction::Scrolled {
                        pane: FocusedPane::Editor,
                        delta: -1,
                    };
                }
                KeyCode::Down => {
                    self.editor_state.move_cursor_down();
                    return AppAction::Scrolled {
                        pane: FocusedPane::Editor,
                        delta: 1,
                    };
                }
                KeyCode::Home => {
                    self.editor_state.move_to_line_start();
                    return AppAction::None;
                }
                KeyCode::End => {
                    self.editor_state.move_to_line_end();
                    return AppAction::None;
                }
                KeyCode::PageUp => {
                    self.editor_state.scroll_up(8);
                    return AppAction::Scrolled {
                        pane: FocusedPane::Editor,
                        delta: -8,
                    };
                }
                KeyCode::PageDown => {
                    self.editor_state.scroll_down(8);
                    return AppAction::Scrolled {
                        pane: FocusedPane::Editor,
                        delta: 8,
                    };
                }
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    self.editor_state.insert_char(c);
                    return AppAction::BufferModified;
                }
                _ => return AppAction::None,
            }
        }

        // Standard navigation & exit keybindings for Roadmap, Task, and Guidance panes
        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
            self.should_quit = true;
            return AppAction::Quit;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up();
                return AppAction::Scrolled {
                    pane: self.focused_pane,
                    delta: -1,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down();
                return AppAction::Scrolled {
                    pane: self.focused_pane,
                    delta: 1,
                };
            }
            KeyCode::Enter => {
                let submission = self.submit_code();
                if let LearningState::Examining {
                    submission: sub, ..
                } = &mut ctx.state
                {
                    *sub = Some(submission.clone());
                }
                return AppAction::Submit(submission);
            }
            _ => {}
        }

        AppAction::None
    }

    /// Dispatches an `AppEvent` into state mutations and action responses.
    pub fn handle_event(&mut self, event: AppEvent, ctx: &mut StateMachineContext) -> AppAction {
        match event {
            AppEvent::Key(key) => self.handle_key(key, ctx),
            AppEvent::Tick => {
                self.tick();
                AppAction::None
            }
            AppEvent::Resize(_, _) => AppAction::None,
        }
    }

    /// Runs the complete interactive event loop, redrawing on each tick or keypress.
    pub async fn run_loop<B: Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
        events: &mut EventHandler,
        ctx: &mut StateMachineContext,
    ) -> Result<AppAction, Box<dyn std::error::Error>> {
        self.sync_focus_states();

        while !self.should_quit {
            terminal.draw(|f| {
                self.draw_mut(f, ctx);
            })?;

            if let Some(event) = events.next().await {
                let action = self.handle_event(event, ctx);
                if action == AppAction::Quit || self.should_quit {
                    return Ok(AppAction::Quit);
                }
                if let AppAction::Submit(ref _code) = action {
                    return Ok(action);
                }
            }
        }

        Ok(AppAction::Quit)
    }

    /// Renders all 4 panes, status bar, and any modal overlays.
    pub fn draw_mut(&mut self, frame: &mut Frame, ctx: &StateMachineContext) {
        self.sync_focus_states();
        let area = frame.size();

        // 0. Compute layout with bottom status bar
        let (cockpit_area, status_area) = if area.height >= 4 {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(1)])
                .split(area);
            (chunks[0], chunks[1])
        } else {
            (area, Rect::default())
        };

        let panes = calculate_cockpit_layout(cockpit_area);

        // 1. Roadmap Pane
        let is_roadmap_focused = self.focused_pane == FocusedPane::Roadmap;
        let roadmap_title = if is_roadmap_focused {
            Line::from(vec![
                Span::raw(" 🗺️ Roadmap "),
                Span::styled(
                    "[ACTIVE]",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from(vec![Span::raw(" 🗺️ Roadmap ")])
        };

        let roadmap_border_style = if is_roadmap_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let roadmap_block = Block::default()
            .title(roadmap_title)
            .borders(Borders::ALL)
            .border_style(roadmap_border_style);

        let roadmap_widget = RoadmapWidget::new(&ctx.roadmap.topics, ctx.current_topic_index)
            .block(roadmap_block)
            .focused(is_roadmap_focused);

        frame.render_stateful_widget(roadmap_widget, panes.roadmap, &mut self.roadmap_state);

        // 2. Challenge & Theory Pane (or Diagnostic Quiz Widget during Diagnostic state)
        let is_task_focused = self.focused_pane == FocusedPane::Task;
        let task_border_style = if is_task_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        if let LearningState::Diagnostic {
            questions,
            current_index,
            ..
        } = &ctx.state
        {
            if let Some(q) = questions.get(*current_index) {
                let quiz_title = if is_task_focused {
                    Line::from(vec![
                        Span::raw(" 🔍 Baseline Diagnostic "),
                        Span::styled(
                            "[ACTIVE]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else {
                    Line::from(vec![Span::raw(" 🔍 Baseline Diagnostic ")])
                };

                let quiz_block = Block::default()
                    .title(quiz_title)
                    .borders(Borders::ALL)
                    .border_style(task_border_style);

                let quiz_widget = DiagnosticQuizWidget::new(q, *current_index, questions.len())
                    .block(quiz_block)
                    .focused(is_task_focused);

                frame.render_stateful_widget(quiz_widget, panes.task, &mut self.quiz_state);
            } else {
                let task_title = if is_task_focused {
                    Line::from(vec![
                        Span::raw(" 🔍 Baseline Diagnostic "),
                        Span::styled(
                            "[ACTIVE]",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else {
                    Line::from(vec![Span::raw(" 🔍 Baseline Diagnostic ")])
                };

                let task_block = Block::default()
                    .title(task_title)
                    .borders(Borders::ALL)
                    .border_style(task_border_style);

                let theory_widget = TheoryViewerWidget::new(
                    "🔍 Baseline diagnostic quiz in progress. Analyzing responses...",
                    self.task_scroll,
                )
                .block(task_block)
                .focused(is_task_focused);

                frame.render_widget(theory_widget, panes.task);
            }
        } else {
            let task_title = if is_task_focused {
                Line::from(vec![
                    Span::raw(" 🧪 Challenge & Theory "),
                    Span::styled(
                        "[ACTIVE]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![Span::raw(" 🧪 Challenge & Theory ")])
            };

            let task_text = match &ctx.state {
                LearningState::Tutoring { material, task, .. } => {
                    format!("# Theory & Conceptual Framework\n\n{material}\n\n---\n\n## 🎯 Challenge Assignment\n\n{task}")
                }
                LearningState::Examining {
                    task,
                    submission,
                    assessment,
                    ..
                } => {
                    let mut content = format!("## 🎯 Active Challenge\n\n{task}\n\n---\n");
                    if let Some(sub) = submission {
                        content.push_str(&format!(
                            "### Submitted Code Buffer\n```rust\n{sub}\n```\n\n"
                        ));
                    }
                    if let Some(a) = assessment {
                        content.push_str(&format!(
                            "### Socratic Evaluation\n- Verdict: {:?}\n- Score: {:.1}%\n\n> {}\n",
                            a.verdict, a.score, a.feedback
                        ));
                    } else {
                        content.push_str(
                            "Waiting for student code submission...\nPress [Enter] to submit code.",
                        );
                    }
                    content
                }
                LearningState::Coaching { weakness_report } => {
                    let mut text = "# 🧭 Personalized Roadmap Formulation\n\nSynthesizing curriculum from diagnostic report...\n\n".to_string();
                    if !weakness_report.weak_areas.is_empty() {
                        text.push_str("### Identified Growth Areas:\n");
                        for area in &weakness_report.weak_areas {
                            text.push_str(&format!("- {area}\n"));
                        }
                    }
                    text
                }
                LearningState::Completed => {
                    "🎉 All roadmap milestones successfully mastered! Curriculum completed."
                        .to_string()
                }
                LearningState::Diagnostic { .. } => unreachable!(),
            };

            let task_block = Block::default()
                .title(task_title)
                .borders(Borders::ALL)
                .border_style(task_border_style);

            let theory_widget = TheoryViewerWidget::new(&task_text, self.task_scroll)
                .block(task_block)
                .focused(is_task_focused);

            frame.render_widget(theory_widget, panes.task);
        }

        // 3. Socratic Guidance Pane (with Animated Lighthouse Widget)
        let is_guidance_focused = self.focused_pane == FocusedPane::Guidance;
        let guidance_title = if is_guidance_focused {
            Line::from(vec![
                Span::raw(" 💡 Socratic Guidance "),
                Span::styled(
                    "[ACTIVE]",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from(vec![Span::raw(" 💡 Socratic Guidance ")])
        };

        let guidance_border_style = if is_guidance_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let guidance_block = Block::default()
            .title(guidance_title)
            .borders(Borders::ALL)
            .border_style(guidance_border_style);

        let guidance_inner = guidance_block.inner(panes.socratic_guidance);
        frame.render_widget(guidance_block, panes.socratic_guidance);

        if guidance_inner.height >= 12 && guidance_inner.width >= 35 {
            // Split into animated lighthouse on top and Socratic feedback on bottom
            let guidance_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(10), Constraint::Min(3)])
                .split(guidance_inner);

            let lighthouse = LighthouseWidget::new(self.tick_count);
            frame.render_widget(lighthouse, guidance_chunks[0]);

            let hint_text = self.format_guidance_text(ctx);
            let hint_paragraph =
                Paragraph::new(hint_text).style(Style::default().fg(Color::LightCyan));
            frame.render_widget(hint_paragraph, guidance_chunks[1]);
        } else {
            // Compact view: guidance text with beacon indicator
            let hint_text = format!(
                "🗼 Socratic Lighthouse [Frame {}/8]\n\n{}",
                (self.tick_count % 8) + 1,
                self.format_guidance_text(ctx)
            );
            let paragraph = Paragraph::new(hint_text).style(Style::default().fg(Color::LightCyan));
            frame.render_widget(paragraph, guidance_inner);
        }

        // 4. Code Submission Editor Pane
        let is_editor_focused = self.focused_pane == FocusedPane::Editor;
        let editor_title = if is_editor_focused {
            Line::from(vec![
                Span::raw(" 📝 Code Submission "),
                Span::styled(
                    "[ACTIVE]",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from(vec![Span::raw(" 📝 Code Submission ")])
        };

        let editor_border_style = if is_editor_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let editor_block = Block::default()
            .title(editor_title)
            .borders(Borders::ALL)
            .border_style(editor_border_style);

        let editor_widget = EditorWidget::new()
            .block(editor_block)
            .focused(is_editor_focused);

        frame.render_stateful_widget(editor_widget, panes.code_editor, &mut self.editor_state);

        // 5. Persistent Bottom Status Bar Widget
        if status_area.height > 0 {
            let status_widget = StatusBarWidget::new()
                .status_message(self.status_message.as_deref())
                .current_state(Some(ctx.state.name()));
            frame.render_widget(status_widget, status_area);
        }

        // 6. Help Modal Overlay (toggled with '?')
        if self.show_help {
            let popup_area = centered_rect(65, 75, area);
            let help_widget = HelpWidget::new();
            frame.render_widget(help_widget, popup_area);
        }
    }

    /// Immutable draw method implementing backward compatibility.
    pub fn draw(&self, frame: &mut Frame, ctx: &StateMachineContext) {
        let mut cloned = Self {
            focused_pane: self.focused_pane,
            should_quit: self.should_quit,
            tick_count: self.tick_count,
            task_scroll: self.task_scroll,
            guidance_scroll: self.guidance_scroll,
            roadmap_state: self.roadmap_state.clone(),
            editor_state: self.editor_state.clone(),
            quiz_state: self.quiz_state.clone(),
            show_help: self.show_help,
            last_submission: self.last_submission.clone(),
            status_message: self.status_message.clone(),
        };
        cloned.draw_mut(frame, ctx);
    }

    fn sync_focus_states(&mut self) {
        self.roadmap_state.is_focused = self.focused_pane == FocusedPane::Roadmap;
        self.editor_state.is_focused = self.focused_pane == FocusedPane::Editor;
        self.quiz_state.is_focused = self.focused_pane == FocusedPane::Task;
    }

    fn format_guidance_text(&self, ctx: &StateMachineContext) -> String {
        match &ctx.state {
            LearningState::Examining {
                assessment: Some(a),
                ..
            } => {
                format!(
                    "> 🗼 Lighthouse Beacon Feedback:\n{}\n\nScore: {:.1}% | Verdict: {:?}",
                    a.feedback, a.score, a.verdict
                )
            }
            LearningState::Tutoring { retry_count, .. } if *retry_count > 0 => {
                format!("> 💡 Socratic Reflection (Attempt {retry_count}):\nReflect on the compiler diagnostics. Does the lifetime outlive the caller scope?")
            }
            LearningState::Diagnostic { .. } => {
                "> 🗼 Lighthouse Beacon:\nDiagnostic probing in progress. Formulate answers to reveal your baseline knowledge.".to_string()
            }
            LearningState::Coaching { .. } => {
                "> 🗼 Lighthouse Beacon:\nSynthesizing optimal roadmap path. Prioritizing foundational concepts.".to_string()
            }
            LearningState::Completed => {
                "> 🗼 Lighthouse Beacon:\nAll curriculum milestones mastered. The beacon shines bright across your path.".to_string()
            }
            _ => {
                "> 🗼 Lighthouse Beacon:\nRead the theory carefully before starting the challenge. Formulate your solution in the editor and press [Enter].".to_string()
            }
        }
    }
}
