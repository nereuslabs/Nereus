# Contributing to Nereus 2.0

Thank you for your interest in contributing to **Nereus**! Nereus is an autonomous, terminal-native AI programming mentor built entirely in idiomatic Rust.

We welcome contributions from open-source developers worldwide. To maintain architectural integrity, determinism, and code quality, all contributors (both human developers and autonomous AI agents) are expected to adhere to the guidelines set forth in this document and [AGENTS.md](AGENTS.md).

---

## 1. Core Engineering Invariants

1. **Strict Language Rule**: All code identifiers, comments, documentation, commit messages, PR descriptions, and issue discussions must be conducted in **100% idiomatic English**.
2. **Offline-First Verification**: Every change must pass offline test suites (`cargo test` or `StubLLMClient`) before any commit or push is initiated. External network access or paid token consumption is forbidden during automated CI testing.
3. **No Direct Pushes to `main`**: The `main` branch is protected by GitHub repository rulesets and serves as the release gate. All feature branches and pull requests target **`develop`**.
4. **Zero AI Attribution Markers**: Commit messages, PR titles, descriptions, and issues must never include AI badges, watermarks, or AI attribution tags.
5. **Zero Secret Leakage**: API tokens (e.g., `OPENROUTER_API_KEY`, `NEREUS_API_KEY`) must never be written to files, commits, or test outputs.

---

## 2. Conventional Commits Specification

All commit messages must strictly adhere to the [Conventional Commits v1.0.0](https://www.conventionalcommits.org/) specification:

```text
<type>(<scope>): <short summary in imperative mood>

[optional body explaining WHY this change was made and WHAT it achieves]

[optional footer(s): issue references, BREAKING CHANGE]
```

### Allowed Commit Types
- `feat`: New feature or agent capability
- `fix`: Bug fix or state machine transition correction
- `refactor`: Code restructuring without behavioral change
- `perf`: Performance improvement or latency reduction
- `test`: Adding or updating hermetic unit/integration tests
- `docs`: Documentation additions or corrections
- `chore`: Build configuration, dependencies, or workspace scaffolding

### Allowed Scopes
- `nereus-core`
- `nereus-engine`
- `nereus-agents`
- `nereus-llm`
- `nereus-sandbox`
- `nereus-storage`
- `nereus-tui`
- `nereus-plugin`
- `nereus-cli`
- `workspace` / `deps`

---

## 3. Git Worktree Isolation Workflow

To prevent compilation lock contention and allow isolated feature development across crates, we strongly encourage using `git worktree`:

```bash
# 1. Create an isolated worktree for your feature
git worktree add worktrees/my-feature -b feat/scope-my-feature

# 2. Enter worktree and perform work
cd worktrees/my-feature

# 3. Test your changes locally
cargo test -p nereus-sandbox

# 4. Commit using Conventional Commits
git add .
git commit -m "feat(nereus-sandbox): implement custom feature"
git push -u origin feat/scope-my-feature

# 5. Clean up after pull request merge
cd ../..
git worktree remove worktrees/my-feature
```

---

## 4. Verification & Quality Gates

Before opening a pull request, verify all quality gates pass locally:

```bash
# 1. Hermetic offline test suite
cargo test --workspace

# 2. Code formatting check
cargo fmt --all --check

# 3. Clippy linter with zero warnings tolerated
cargo clippy --workspace --all-targets -- -D warnings

# 4. Pre-release End-to-End verification protocol
./scripts/verify_e2e.sh
```

---

## 5. Pull Request Protocol

1. **Target Branch**: Always open pull requests targeting `develop`.
2. **Branch Name**: Follow `<type>/<crate-or-scope>-<slug>` (e.g. `feat/sandbox-custom-runner`, `fix/engine-threshold-clamp`).
3. **Issue Linkage**: Every PR must reference or close an existing tracked GitHub issue (e.g., `Closes #123`).
4. **Description**: Follow the standard PR template from [AGENTS.md](AGENTS.md) detailing Summary, Motivation, Key Changes, and Verification checklists.

---

## 6. Architecture & Workspace Structure

Nereus is organized as a decoupled 9-crate Cargo workspace:

* `nereus-core`: Domain models, algebraic FSM state enums, traits (`LlmClient`, `SandboxRunner`, `StorageBackend`).
* `nereus-engine`: Finite state automaton, transitions, dynamic passing threshold evaluator.
* `nereus-agents`: The 6 Socratic pedagogical agents (`DiagnosticAgent`, `CoachAgent`, `TutorAgent`, `ExaminerAgent`, `LibrarianAgent`, `AdversaryAgent`).
* `nereus-llm`: OpenRouter, direct Anthropic (`ClaudeClient`), `OllamaClient`, in-process `LocalCandleClient`, deterministic `StubLLMClient`.
* `nereus-sandbox`: Wasmtime WebAssembly execution runner (fuel + memory capped), `HostSandboxRunner` (`rustc` compiler scratchpad), AST static analysis inspector (`syn::visit`).
* `nereus-storage`: SQLite with WAL mode, FTS5 full-text search, schema migrations, session history.
* `nereus-tui`: Ratatui full-screen interactive cockpit (split panes, rotating Socratic Lighthouse, interactive quiz, undo/redo buffer, search modal, mouse support).
* `nereus-plugin`: `ModuleRegistry` providing dynamic Tier-1 hot-switching and WebAssembly Component Model WIT host.
* `nereus-cli`: Binary entry point, subcommands (`sessions`, `resume`, `bench`, `search`, `sync`).

---

## 7. License

By contributing to Nereus, you agree that your contributions will be licensed under the project's [MIT License](LICENSE).
