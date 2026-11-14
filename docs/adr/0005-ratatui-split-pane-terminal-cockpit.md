# ADR 0005: Ratatui Split-Pane Terminal Cockpit over Web UI

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-11

---

## Context & Problem Statement

Nereus v1 utilized a browser-based user interface (Chainlit). For programming mentorship, browser interfaces create friction:
1. Context switching between developer terminal/editor and browser tabs.
2. High overhead (Node.js/Python server, WebSocket connection, browser rendering engine).
3. Inability to run smoothly over remote SSH sessions or inside headless development containers.

## Decision Drivers

* Terminal-first developer ergonomics: learn and code where developers live.
* Low latency rendering at 60 FPS in standard terminal emulators.
* Zero external browser or webview dependencies.

## Considered Options

1. Modern web frontend (React / Vite + Tauri).
2. Terminal UI using Python `Textual`.
3. Terminal UI using Rust `Ratatui` + `Crossterm`.

## Decision Outcome

Chosen option: **Ratatui Terminal Cockpit (`nereus-tui`)**:
- 4-pane responsive split layout (Theory Viewer, Syntax-Highlighted Code Editor, Dynamic Roadmap Progress, Animated Socratic Lighthouse).
- Interactive widgets: Multi-choice Diagnostic Quiz (`quiz.rs`), in-cockpit code editor buffer (`editor.rs`), status bar (`status_bar.rs`), and modal cheatsheet overlay (`help.rs`).
- Native keyboard navigation (Tab pane cycling, Arrow/Vim scrolling, hotkeys `Ctrl+C`, `?`, `Enter`).

### Consequences

* **Positive**: Operates seamlessly over SSH, tmux, and lightweight terminal environments.
* **Positive**: Sub-millisecond rendering with zero browser memory overhead.
* **Negative**: Rich graphics are constrained to Unicode and ASCII art (e.g., the rotating 360° Socratic Lighthouse).
