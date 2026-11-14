# GEMINI.md — Nereus 2.0 Antigravity Engineering & Development Guide

> **Scope**: Primary agent configuration and workflow guidelines for Google Antigravity operating in Nereus 2.0.

---

## 1. System Architecture Invariants

- **Language & Toolchain**: 100% Idiomatic Rust 2021 edition (MSRV 1.78+).
- **Core Algebraic State Machine**: Compile-time finite state automaton (`enum LearningState` in `nereus-core`).
- **Modular Workspace**: 9 decoupled crates:
  1. `nereus-core`: Domain models, algebraic FSM state enums, traits (`LlmClient`, `SandboxRunner`, `StorageBackend`).
  2. `nereus-engine`: Finite state automaton, transitions, dynamic passing threshold evaluator.
  3. `nereus-agents`: The 4 Socratic pedagogical agents (`DiagnosticAgent`, `CoachAgent`, `TutorAgent`, `ExaminerAgent`).
  4. `nereus-llm`: OpenRouter, direct Anthropic (`ClaudeClient`), deterministic `StubLLMClient`, prompt registry.
  5. `nereus-sandbox`: Wasmtime WebAssembly execution runner (fuel + memory capped), `HostSandboxRunner` (`rustc` compiler scratchpad), and curated `Curriculum`.
  6. `nereus-storage`: SQLite with WAL mode (`PRAGMA journal_mode=WAL`), schema migrations, session history.
  7. `nereus-tui`: Ratatui full-screen interactive cockpit (split panes, rotating Socratic Lighthouse widget, interactive quiz, syntax-highlighted code editor buffer, status bar, cheatsheet overlay).
  8. `nereus-plugin`: `ModuleRegistry` providing dynamic Tier-1 hot-switching across LLM, Sandbox, and Storage providers.
  9. `nereus-cli`: Binary entry point, subcommands (`sessions`, `resume`), offline demo mode.
- **Offline-First / Zero-Token CI**: Deterministic `StubLLMClient` and `MockSandboxRunner` ensure all unit and integration tests run hermetically without external network calls or token expenditure.

---

## 2. Agent Governance & Mandatory Remote Git Lifecycle

Follow all contracts established in `AGENTS.md`:

1. **Strict Language Rule**: 100% English across all identifiers, comments, documentation, commit messages, PR descriptions, and issue discussions.
2. **Conventional Commits**: Every commit must adhere to `<type>(<scope>): <summary>` format (e.g. `feat(nereus-tui): implement quiz widget`).
3. **No Direct Pushes to `main`**: All features and bugfixes target `develop`. The `main` branch is protected by repository rulesets (`id=20506063`) and serves as the release gate.
4. **Zero AI Watermarks**: Never append "Generated with claude/gemini", AI badges, or agent attribution to commit messages, PRs, or issues.
5. **Mandatory Remote Lifecycle**: Never leave changes local-only:
   - Branch: `feat/<crate-or-scope>-<slug>` branched from `develop`.
   - Worktree isolation: Use `worktrees/<module>` for parallel subagent tasks.
   - Remote push: `git push -u origin <branch>`.
   - Issue linkage: File or reference tracked issue (`Closes #<id>`).
   - Pull Request: Open formal PR targeting `develop` with the template from `AGENTS.md`.

---

## 3. Continuous Dual Synchronization Protocol

Whenever a batch of changes lands, a milestone phase transitions, or a PR is created/merged:

### 3.1. Local Kanban (On-Device)
- Resides in `.kanban/` (gitignored).
- Move task: `python3 .kanban/kanban.py move <task-id> <new-status>`
- Render board: `python3 .kanban/kanban.py render` (updates `.kanban/BOARD.md`).

### 3.2. Notion Status Cockpit (Cloud)
- **Page ID**: `41300ced-bff4-8178-a81f-c6d215179c84`
- **Page URL**: `https://app.notion.com/p/41300cedbff48178a81fc6d215179c84?pvs=204`
- Maintain stage statuses and milestone progress.

---

## 4. Verification & Quality Gates

Run these commands before any commit or PR:

```bash
# 1. Hermetic offline test suite
cargo test

# 2. Format check
cargo fmt --all --check

# 3. Clippy linter check (zero warnings tolerated)
cargo clippy --workspace --all-targets -- -D warnings
```

---

## 5. Common CLI Tasks

```bash
# Run in offline deterministic demo mode
cargo run -p nereus-cli -- --offline --demo

# Resume previous learning session
cargo run -p nereus-cli -- resume

# List all past sessions
cargo run -p nereus-cli -- sessions list
```
