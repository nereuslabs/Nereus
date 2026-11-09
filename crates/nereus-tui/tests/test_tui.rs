//! Comprehensive unit and integration test suite for Nereus TUI cockpit.
//!
//! Verifies:
//! 1. Layout calculations and split-pane bounding boxes
//! 2. Focus cycling and pane navigation
//! 3. Keyboard event handling (Tab, Esc, q, Up/Down, j/k, Enter)
//! 4. Animated Socratic Lighthouse widget 360-degree rotation
//! 5. Code Submission Editor syntax highlighting and cursor navigation
//! 6. Roadmap progress bar calculation and topic scrolling
//! 7. End-to-end rendering across all compile-time algebraic FSM states
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use nereus_core::models::{
    Assessment, DiagnosticQuestion, Roadmap, RoadmapTopic, UserLevel, UserProfile, Verdict,
    WeaknessReport,
};
use nereus_core::state::{LearningState, StateMachineContext};
use nereus_tui::cockpit::{CockpitApp, FocusedPane};
use nereus_tui::events::{AppAction, AppEvent, EventHandler};
use nereus_tui::layout::calculate_cockpit_layout;
use nereus_tui::widgets::{
    highlight_rust_line, EditorState, LighthouseWidget, RoadmapState, RoadmapWidget,
    TOTAL_LIGHTHOUSE_FRAMES,
};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use uuid::Uuid;

fn make_key_event(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

fn make_ctrl_key_event(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

fn make_test_context() -> StateMachineContext {
    let profile = UserProfile {
        skill: "Rust".to_string(),
        current_level: UserLevel::Intermediate,
        target_level: UserLevel::Advanced,
        hours_per_day: 2.0,
        deadline_days: 14,
        goal: "Master async Rust and lifetimes".to_string(),
    };

    let topics = vec![
        RoadmapTopic {
            id: "topic-1".to_string(),
            title: "Ownership & Borrowing Invariants".to_string(),
            description: "Deep dive into affine type systems and borrow checker rules".to_string(),
            difficulty: 0.4,
            prerequisites: vec![],
            estimated_hours: 2.0,
        },
        RoadmapTopic {
            id: "topic-2".to_string(),
            title: "Async Futures & Pinning".to_string(),
            description: "Understand Pin, Unpin, and manual Future polling contracts".to_string(),
            difficulty: 0.7,
            prerequisites: vec!["topic-1".to_string()],
            estimated_hours: 3.5,
        },
        RoadmapTopic {
            id: "topic-3".to_string(),
            title: "Lock-Free Concurrency & Atomics".to_string(),
            description: "Memory ordering semantics: Acquire, Release, and SeqCst".to_string(),
            difficulty: 0.9,
            prerequisites: vec!["topic-2".to_string()],
            estimated_hours: 4.0,
        },
    ];

    let roadmap = Roadmap { topics };

    let state = LearningState::Tutoring {
        topic: roadmap.topics[0].clone(),
        material: "Affine types guarantee single ownership.".to_string(),
        task: "Implement a thread-safe reference counted container.".to_string(),
        retry_count: 0,
    };

    StateMachineContext {
        session_id: Uuid::new_v4(),
        profile,
        roadmap,
        current_topic_index: 0,
        state,
    }
}

// ============================================================================
// 1. Layout Calculations & Pane Bounding Boxes
// ============================================================================

#[test]
fn test_layout_calculation_standard_terminal() {
    let area = Rect::new(0, 0, 100, 40);
    let panes = calculate_cockpit_layout(area);

    // Verify all panes fit within the bounding area
    assert!(panes.roadmap.width > 0 && panes.roadmap.height > 0);
    assert!(panes.task.width > 0 && panes.task.height > 0);
    assert!(panes.socratic_guidance.width > 0 && panes.socratic_guidance.height > 0);
    assert!(panes.code_editor.width > 0 && panes.code_editor.height > 0);

    // Verify horizontal alignment on top row
    assert_eq!(panes.roadmap.y, 0);
    assert_eq!(panes.task.y, 0);
    assert_eq!(panes.roadmap.x + panes.roadmap.width, panes.task.x);
    assert_eq!(panes.roadmap.width + panes.task.width, area.width);

    // Verify horizontal alignment on bottom row
    assert_eq!(panes.socratic_guidance.y, panes.roadmap.height);
    assert_eq!(panes.code_editor.y, panes.task.height);
    assert_eq!(
        panes.socratic_guidance.x + panes.socratic_guidance.width,
        panes.code_editor.x
    );
    assert_eq!(
        panes.socratic_guidance.width + panes.code_editor.width,
        area.width
    );

    // Verify vertical totals
    assert_eq!(
        panes.roadmap.height + panes.socratic_guidance.height,
        area.height
    );
    assert_eq!(panes.task.height + panes.code_editor.height, area.height);
}

#[test]
fn test_layout_calculation_proportions() {
    let area = Rect::new(0, 0, 100, 100);
    let panes = calculate_cockpit_layout(area);

    // 55% top, 45% bottom
    assert_eq!(panes.roadmap.height, 55);
    assert_eq!(panes.socratic_guidance.height, 45);

    // 40% left, 60% right
    assert_eq!(panes.roadmap.width, 40);
    assert_eq!(panes.task.width, 60);
    assert_eq!(panes.socratic_guidance.width, 40);
    assert_eq!(panes.code_editor.width, 60);
}

#[test]
fn test_layout_calculation_zero_and_minimal_areas() {
    let zero_area = Rect::new(0, 0, 0, 0);
    let panes_zero = calculate_cockpit_layout(zero_area);
    assert_eq!(panes_zero.roadmap.width, 0);
    assert_eq!(panes_zero.code_editor.height, 0);

    let tiny_area = Rect::new(0, 0, 10, 5);
    let panes_tiny = calculate_cockpit_layout(tiny_area);
    assert!(panes_tiny.roadmap.width <= 10);
    assert!(panes_tiny.code_editor.height <= 5);
}

// ============================================================================
// 2. Focus Management & Pane Navigation
// ============================================================================

#[test]
fn test_focus_cycle_forward() {
    let mut app = CockpitApp::new();
    assert_eq!(app.focused_pane, FocusedPane::Roadmap);

    app.cycle_focus();
    assert_eq!(app.focused_pane, FocusedPane::Task);

    app.cycle_focus();
    assert_eq!(app.focused_pane, FocusedPane::Guidance);

    app.cycle_focus();
    assert_eq!(app.focused_pane, FocusedPane::Editor);

    app.cycle_focus();
    assert_eq!(app.focused_pane, FocusedPane::Roadmap);
}

#[test]
fn test_focus_cycle_backward() {
    let mut app = CockpitApp::new();
    assert_eq!(app.focused_pane, FocusedPane::Roadmap);

    app.cycle_focus_reverse();
    assert_eq!(app.focused_pane, FocusedPane::Editor);

    app.cycle_focus_reverse();
    assert_eq!(app.focused_pane, FocusedPane::Guidance);

    app.cycle_focus_reverse();
    assert_eq!(app.focused_pane, FocusedPane::Task);

    app.cycle_focus_reverse();
    assert_eq!(app.focused_pane, FocusedPane::Roadmap);
}

#[test]
fn test_direct_set_focus() {
    let mut app = CockpitApp::new();
    app.set_focus(FocusedPane::Guidance);
    assert_eq!(app.focused_pane, FocusedPane::Guidance);

    app.set_focus(FocusedPane::Editor);
    assert_eq!(app.focused_pane, FocusedPane::Editor);
    assert!(app.editor_state.is_focused);
    assert!(!app.roadmap_state.is_focused);
}

#[test]
fn test_pane_titles() {
    assert_eq!(FocusedPane::Roadmap.title(), "Roadmap");
    assert_eq!(FocusedPane::Task.title(), "Challenge & Theory");
    assert_eq!(FocusedPane::Guidance.title(), "Socratic Guidance");
    assert_eq!(FocusedPane::Editor.title(), "Code Submission");
}

// ============================================================================
// 3. Interactive Keyboard Event Handling
// ============================================================================

#[test]
fn test_tab_key_cycles_focus() {
    let mut app = CockpitApp::new();
    let mut ctx = make_test_context();

    let tab_event = make_key_event(KeyCode::Tab);
    let action = app.handle_key(tab_event, &mut ctx);

    assert_eq!(action, AppAction::FocusChanged(FocusedPane::Task));
    assert_eq!(app.focused_pane, FocusedPane::Task);
}

#[test]
fn test_backtab_key_cycles_focus_reverse() {
    let mut app = CockpitApp::new();
    let mut ctx = make_test_context();

    let backtab_event = make_key_event(KeyCode::BackTab);
    let action = app.handle_key(backtab_event, &mut ctx);

    assert_eq!(action, AppAction::FocusChanged(FocusedPane::Editor));
    assert_eq!(app.focused_pane, FocusedPane::Editor);
}

#[test]
fn test_quit_keybindings() {
    let mut app = CockpitApp::new();
    let mut ctx = make_test_context();

    // 'q' key quits
    let q_event = make_key_event(KeyCode::Char('q'));
    let action = app.handle_key(q_event, &mut ctx);
    assert_eq!(action, AppAction::Quit);
    assert!(app.should_quit);

    // Reset and test Esc key
    app.should_quit = false;
    let esc_event = make_key_event(KeyCode::Esc);
    let action = app.handle_key(esc_event, &mut ctx);
    assert_eq!(action, AppAction::Quit);
    assert!(app.should_quit);

    // Reset and test Ctrl+C
    app.should_quit = false;
    let ctrl_c = make_ctrl_key_event(KeyCode::Char('c'));
    let action = app.handle_key(ctrl_c, &mut ctx);
    assert_eq!(action, AppAction::Quit);
    assert!(app.should_quit);
}

#[test]
fn test_scroll_keybindings_all_panes() {
    let mut app = CockpitApp::new();
    let mut ctx = make_test_context();

    // 1. Roadmap scrolling
    app.set_focus(FocusedPane::Roadmap);
    let down_event = make_key_event(KeyCode::Down);
    let action = app.handle_key(down_event, &mut ctx);
    assert_eq!(
        action,
        AppAction::Scrolled {
            pane: FocusedPane::Roadmap,
            delta: 1
        }
    );

    let up_event = make_key_event(KeyCode::Up);
    let action = app.handle_key(up_event, &mut ctx);
    assert_eq!(
        action,
        AppAction::Scrolled {
            pane: FocusedPane::Roadmap,
            delta: -1
        }
    );

    // 2. Task scrolling via j/k
    app.set_focus(FocusedPane::Task);
    let j_event = make_key_event(KeyCode::Char('j'));
    let action = app.handle_key(j_event, &mut ctx);
    assert_eq!(
        action,
        AppAction::Scrolled {
            pane: FocusedPane::Task,
            delta: 1
        }
    );
    assert_eq!(app.task_scroll, 1);

    let k_event = make_key_event(KeyCode::Char('k'));
    let action = app.handle_key(k_event, &mut ctx);
    assert_eq!(
        action,
        AppAction::Scrolled {
            pane: FocusedPane::Task,
            delta: -1
        }
    );
    assert_eq!(app.task_scroll, 0);

    // 3. Guidance scrolling
    app.set_focus(FocusedPane::Guidance);
    app.handle_key(make_key_event(KeyCode::Down), &mut ctx);
    assert_eq!(app.guidance_scroll, 1);
    app.handle_key(make_key_event(KeyCode::Up), &mut ctx);
    assert_eq!(app.guidance_scroll, 0);

    // 4. Editor scrolling
    app.set_focus(FocusedPane::Editor);
    app.editor_state.lines = vec!["line1".into(), "line2".into(), "line3".into()];
    app.handle_key(make_key_event(KeyCode::Down), &mut ctx);
    assert_eq!(app.editor_state.cursor_line, 1);
    app.handle_key(make_key_event(KeyCode::Up), &mut ctx);
    assert_eq!(app.editor_state.cursor_line, 0);
}

#[test]
fn test_enter_key_submits_code() {
    let mut app = CockpitApp::with_code("pub fn custom_solution() -> bool { true }");
    let mut ctx = make_test_context();

    let enter_event = make_key_event(KeyCode::Enter);
    let action = app.handle_key(enter_event, &mut ctx);

    assert_eq!(
        action,
        AppAction::Submit("pub fn custom_solution() -> bool { true }".to_string())
    );
    assert_eq!(
        app.last_submission,
        Some("pub fn custom_solution() -> bool { true }".to_string())
    );
}

// ============================================================================
// 4. Animated Socratic Lighthouse Widget
// ============================================================================

#[test]
fn test_lighthouse_360_degree_rotation() {
    assert_eq!(TOTAL_LIGHTHOUSE_FRAMES, 8);

    for tick in 0..16 {
        let widget = LighthouseWidget::new(tick);
        let frame = widget.frame_index();
        assert_eq!(frame, (tick as usize) % 8);

        let lines = widget.render_lines();
        assert!(!lines.is_empty());
        assert!(!widget.beam_direction().is_empty());
    }

    // Verify directional beam names
    assert!(LighthouseWidget::new(0).beam_direction().contains("East"));
    assert!(LighthouseWidget::new(1)
        .beam_direction()
        .contains("South-East"));
    assert!(LighthouseWidget::new(2).beam_direction().contains("South"));
    assert!(LighthouseWidget::new(3)
        .beam_direction()
        .contains("South-West"));
    assert!(LighthouseWidget::new(4).beam_direction().contains("West"));
    assert!(LighthouseWidget::new(5)
        .beam_direction()
        .contains("North-West"));
    assert!(LighthouseWidget::new(6).beam_direction().contains("North"));
    assert!(LighthouseWidget::new(7)
        .beam_direction()
        .contains("North-East"));
}

#[test]
fn test_lighthouse_render_into_buffer() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let lighthouse = LighthouseWidget::new(2).with_status("Socratic Alignment Active");
            f.render_widget(lighthouse, f.size());
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text = buffer
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    assert!(text.contains("S O C R A T I C   C L A R I T Y") || text.contains("▄█▄"));
}

// ============================================================================
// 5. Code Submission Editor & Syntax Highlighting
// ============================================================================

#[test]
fn test_syntax_highlighting_rust_keywords() {
    let line = "pub async fn solve() -> Result<(), Box<dyn Error>> {";
    let spans = highlight_rust_line(line);
    assert!(!spans.is_empty());

    let has_keyword = spans
        .iter()
        .any(|s| s.content == "pub" || s.content == "async" || s.content == "fn");
    assert!(has_keyword);

    let has_type = spans
        .iter()
        .any(|s| s.content == "Result" || s.content == "Box");
    assert!(has_type);
}

#[test]
fn test_syntax_highlighting_comments_and_strings() {
    let comment_line = "    // This is a Socratic hint";
    let comment_spans = highlight_rust_line(comment_line);
    assert!(comment_spans
        .iter()
        .any(|s| s.content.contains("// This is a Socratic hint")));

    let string_line = r#"let msg = "Hello Socratic Nereus";"#;
    let string_spans = highlight_rust_line(string_line);
    assert!(string_spans
        .iter()
        .any(|s| s.content == "\"Hello Socratic Nereus\""));
}

#[test]
fn test_editor_buffer_mutations() {
    let mut state = EditorState::new("");
    assert_eq!(state.get_code(), "");

    state.insert_char('f');
    state.insert_char('n');
    state.insert_char(' ');
    assert_eq!(state.get_code(), "fn ");

    state.insert_newline();
    state.insert_char('}');
    assert_eq!(state.get_code(), "fn \n}");

    state.delete_backspace();
    assert_eq!(state.get_code(), "fn \n");

    state.delete_backspace();
    assert_eq!(state.get_code(), "fn ");
}

#[test]
fn test_editor_cursor_navigation() {
    let mut state = EditorState::new("line one\nline two\nline three");
    assert_eq!(state.cursor_line, 0);
    assert_eq!(state.cursor_col, 0);

    state.move_cursor_down();
    assert_eq!(state.cursor_line, 1);

    state.move_cursor_right();
    assert_eq!(state.cursor_col, 1);

    state.move_to_line_end();
    assert_eq!(state.cursor_col, "line two".len());

    state.move_to_line_start();
    assert_eq!(state.cursor_col, 0);

    state.move_cursor_up();
    assert_eq!(state.cursor_line, 0);
}

// ============================================================================
// 6. Roadmap Mastery Calculation & Widget Rendering
// ============================================================================

#[test]
fn test_roadmap_progress_percentage() {
    let topics = vec![
        RoadmapTopic {
            id: "1".into(),
            title: "T1".into(),
            description: "D1".into(),
            difficulty: 0.1,
            prerequisites: vec![],
            estimated_hours: 1.0,
        },
        RoadmapTopic {
            id: "2".into(),
            title: "T2".into(),
            description: "D2".into(),
            difficulty: 0.2,
            prerequisites: vec![],
            estimated_hours: 1.0,
        },
    ];

    let widget_0 = RoadmapWidget::new(&topics, 0);
    assert_eq!(widget_0.completion_percentage(), 0);

    let widget_1 = RoadmapWidget::new(&topics, 1);
    assert_eq!(widget_1.completion_percentage(), 50);

    let widget_2 = RoadmapWidget::new(&topics, 2);
    assert_eq!(widget_2.completion_percentage(), 100);
}

#[test]
fn test_roadmap_selection_navigation() {
    let mut state = RoadmapState::new();
    assert_eq!(state.selected_index, 0);

    state.select_next(3);
    assert_eq!(state.selected_index, 1);

    state.select_next(3);
    assert_eq!(state.selected_index, 2);

    // Boundary check
    state.select_next(3);
    assert_eq!(state.selected_index, 2);

    state.select_prev();
    assert_eq!(state.selected_index, 1);

    state.select_prev();
    assert_eq!(state.selected_index, 0);

    state.select_prev();
    assert_eq!(state.selected_index, 0);
}

// ============================================================================
// 7. Full Cockpit State Machine Rendering Verification
// ============================================================================

#[test]
fn test_render_tutoring_state() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = CockpitApp::new();
    let ctx = make_test_context();

    terminal
        .draw(|f| {
            app.draw_mut(f, &ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text = buffer
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    assert!(text.contains("Roadmap"));
    assert!(text.contains("Challenge & Theory"));
    assert!(text.contains("Socratic Guidance"));
    assert!(text.contains("Code Submission"));
    assert!(text.contains("Ownership & Borrowing Invariants"));
}

#[test]
fn test_render_examining_state_with_assessment() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let app = CockpitApp::new();
    let mut ctx = make_test_context();

    ctx.state = LearningState::Examining {
        topic: ctx.roadmap.topics[0].clone(),
        task: "Implement thread-safe Arc".to_string(),
        submission: Some("pub struct Arc<T> { ... }".to_string()),
        assessment: Some(Assessment {
            topic_id: "topic-1".to_string(),
            score: 95.0,
            verdict: Verdict::Pass,
            feedback: "Flawless invariant preservation!".to_string(),
            weak_areas: vec![],
        }),
    };

    terminal
        .draw(|f| {
            app.draw(f, &ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text = buffer
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    assert!(text.contains("Flawless invariant preservation!"));
    assert!(text.contains("Pass"));
}

#[test]
fn test_render_diagnostic_state() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = CockpitApp::new();
    let mut ctx = make_test_context();

    ctx.state = LearningState::Diagnostic {
        questions: vec![DiagnosticQuestion {
            id: "diag-1".to_string(),
            question: "Which trait bounds asynchronous stream items?".to_string(),
            options: vec![
                "Stream".to_string(),
                "Future".to_string(),
                "Send".to_string(),
            ],
            correct_index: Some(0),
        }],
        current_index: 0,
        answers: vec![],
    };

    terminal
        .draw(|f| {
            app.draw_mut(f, &ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text = buffer
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();

    assert!(text.contains("Baseline Diagnostic"));
    assert!(text.contains("Which trait bounds asynchronous stream items?"));
}

#[test]
fn test_render_coaching_and_completed_states() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = CockpitApp::new();
    let mut ctx = make_test_context();

    // Coaching
    ctx.state = LearningState::Coaching {
        weakness_report: WeaknessReport {
            weak_areas: vec!["Interior Mutability".to_string()],
            recommended_topics: vec!["UnsafeCell & Cell".to_string()],
        },
    };

    terminal
        .draw(|f| {
            app.draw_mut(f, &ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text = buffer
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("Personalized Roadmap Formulation"));
    assert!(text.contains("Interior Mutability"));

    // Completed
    ctx.state = LearningState::Completed;
    terminal
        .draw(|f| {
            app.draw_mut(f, &ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text = buffer
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("All roadmap milestones successfully mastered"));
}

#[test]
fn test_event_handler_channel() {
    let (mut handler, sender) = EventHandler::new_manual();

    sender
        .try_send(AppEvent::Key(make_key_event(KeyCode::Tab)))
        .unwrap();
    sender.try_send(AppEvent::Tick).unwrap();

    let ev1 = handler.try_next().unwrap();
    assert_eq!(ev1, AppEvent::Key(make_key_event(KeyCode::Tab)));

    let ev2 = handler.try_next().unwrap();
    assert_eq!(ev2, AppEvent::Tick);
}
