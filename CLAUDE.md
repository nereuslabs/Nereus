# CLAUDE.md — Nereus 2.0 Development Guide

## Architecture
- **Language**: Idiomatic Rust 2021 (MSRV 1.78+)
- **State Machine**: Compile-time algebraic finite state automaton (`enum LearningState`)
- **Workspace**: 9 decoupled crates (`nereus-core`, `nereus-engine`, `nereus-agents`, `nereus-llm`, `nereus-sandbox`, `nereus-storage`, `nereus-tui`, `nereus-plugin`, `nereus-cli`)
- **Offline-First**: Deterministic `StubLLMClient` + `MockSandboxRunner` for zero-token CI and test execution
- **Dynamic Hot-Switching**: `ModuleRegistry` (in `nereus-plugin`) for live runtime swapping of LLM, Sandbox, and Storage providers

## Git & GitHub Workflow
- **Conventional Commits**: Every commit must follow `<type>(<scope>): <summary>` (see `AGENTS.md`).
- **Mandatory Remote Lifecycle**: Never keep changes local-only. Push feature branches to GitHub, open tracked Issues (`gh issue create`), and create Pull Requests (`gh pr create`) targeting `develop`.
- **No AI Watermarks**: Never include AI badges, watermarks, or AI attribution in commit messages, PRs, or issues.
- **Feature Branches**: Branch from `develop` using `feat/<crate>-<name>` or `fix/<crate>-<name>`.
- **Integration Lane**: `develop` is the fast-lane branch where all PRs merge with CI checks.
- **Release Gate**: `main` is protected by repository rulesets (`id=20506063`). Releases are tagged SemVer `vX.Y.Z` on `main`.
- **Worktree-Per-Module**: Develop crate features in isolated git worktrees (`worktrees/<module>`).
- **Language Policy**: 100% English across code, comments, commit messages, PRs, and documentation.

## Continuous Dual Synchronization Protocol
Whenever a batch of changes lands, a milestone phase transitions, or a PR is created:
1. **Local Kanban (On-Device)**:
   - Resides in `.kanban/` (gitignored).
   - Update task status: `python3 .kanban/kanban.py move <task-id> <new-status>`.
   - Re-render: `python3 .kanban/kanban.py render` (updates `.kanban/BOARD.md`).
2. **Notion Status Cockpit (Cloud)**:
   - **Page ID**: `41300ced-bff4-8178-a81f-c6d215179c84`
   - **Page URL**: `https://app.notion.com/p/41300cedbff48178a81fc6d215179c84?pvs=204`
   - Update via Notion MCP (`update_page`) to reflect current phase and task states.

## Testing & Verification
```bash
# Offline unit & integration tests
cargo test

# Single crate test
cargo test -p nereus-engine
cargo test -p nereus-core

# Linting & Formatting quality gates
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## Common CLI Tasks
```bash
# Run in offline deterministic demo mode
cargo run -p nereus-cli -- --offline --demo

# Run with custom skill and live OpenRouter inference
cargo run -p nereus-cli -- --skill "Rust" --level "intermediate"
```
