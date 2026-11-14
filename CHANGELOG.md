# Changelog

All notable changes to the **Nereus** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.3.0] - 2026-11-14

### Added
- **AST-Level Static Code Analysis & Anti-Pattern Inspector** (`nereus-sandbox`):
  - In-memory AST traversal engine powered by `syn::visit` with `span-locations`.
  - Proactive static detection of redundant `.clone()` invocations, unguarded `.unwrap()` calls, unencapsulated `unsafe` blocks, and C-style index loops (`for i in 0..v.len()`).
  - Integrated Socratic remediation notes seamlessly passed to `ExaminerAgent` alongside dynamic compiler diagnostics.
- **In-Process Local LLM Inference Engine** (`nereus-llm`):
  - Pure-Rust `LocalCandleClient` utilizing Hugging Face's `candle` tensor framework for zero-cloud, fully offline local inference.
  - Native `OllamaClient` communicating directly over HTTP with local Ollama daemons (`localhost:11434`).
  - Dynamic runtime provider selection via `nereus provider` and in-cockpit hot-switching (`Ctrl+P` / `F2`).
- **Librarian Agent & Semantic Knowledge Retrieval** (`nereus-agents`):
  - Offline searchable knowledge corpus covering Rust standard library primitives (`Result`, `Option`, `Vec`, `Box`, `Pin`, `Arc`, `Mutex`) and the Rustonomicon.
  - Rust compiler diagnostic citation mapper (`E0382`, `E0597`, `E0502`, `E0106`, `E0277`) returning direct book references and conceptual explanations.
- **Adversary Agent & Property-Based Educational Fuzzing** (`nereus-agents`):
  - Automated edge-case generator synthesizing boundary inputs (`i64::MIN`, `i64::MAX`), empty collections, null bytes, and malformed strings.
  - Concurrency stress test harnesses (`std::thread::scope`, high-frequency contention) and unsafe memory stress suites for systems-level curriculum modules.
- **Ratatui Cockpit Editor Polish & UX Refinements** (`nereus-tui`):
  - Unbounded Undo/Redo ring buffer (`Ctrl+Z`, `Ctrl+Y`) tracking non-linear buffer mutations.
  - Dynamic line numbering with active line highlighting and gutter margins.
  - In-cockpit search modal (`Ctrl+F`, `Esc`, `Enter`) with real-time case-insensitive occurrence highlighting.
  - Crossterm mouse integration supporting click-to-focus and mouse wheel scrolling across all cockpit panes.
- **SQLite FTS5 Full-Text Search & Git Journal Synchronization** (`nereus-storage`, `nereus-cli`):
  - SQLite FTS5 extension integration with migration `0004_create_fts_assessment_history.sql`.
  - Top-level CLI command `nereus search <query>` with formatted ASCII output tables and snippet matching.
  - Top-level CLI command `nereus sync` supporting automated bi-directional Git synchronization of user learning journals and assessment logs.
- **WebAssembly Component Model & WIT Plugin Host** (`nereus-plugin`):
  - Canonical WIT specification (`nereus:curriculum@0.3.0`) enabling language-agnostic third-party curriculum development.
  - `WasmCurriculumComponent` loader and runner validating standard WASM binary headers (`\0asm`) and JSON sidecar manifests.
  - Dynamic plugin discovery in `~/.nereus/plugins`.
- **Architecture Decision Records (ADRs)**:
  - `ADR 0011`: Tree-Sitter & Syn AST Traversal for Anti-Pattern Inspection.
  - `ADR 0012`: In-Process Local LLM Inference via Candle and Ollama.
  - `ADR 0013`: WebAssembly Component Model & WIT Interface Types for Extensible Curriculums.
  - `ADR 0014`: Specialized Multi-Agent Expansion (Librarian & Adversary).

### Changed
- Expanded pre-release verification protocol (`scripts/verify_e2e.sh`) from 7 to 10 automated quality gates.
- Bumped workspace packages and npm distribution package to `0.3.0`.
- Refined SQLite database connection initialization to safely configure WAL mode across concurrent handles.

---

## [0.2.0] - 2026-11-14

### Added
- **Initial Public Release of Nereus 2.0 MVP in Pure Rust**:
  - 9-crate Cargo workspace (`nereus-core`, `nereus-engine`, `nereus-agents`, `nereus-llm`, `nereus-sandbox`, `nereus-storage`, `nereus-tui`, `nereus-plugin`, `nereus-cli`).
  - Algebraic finite state machine (`LearningState`) governing deterministic pedagogical transitions.
  - 4 core Socratic agents: `DiagnosticAgent`, `CoachAgent` (topological DAG roadmap), `TutorAgent`, and `ExaminerAgent`.
  - Hermetic execution sandboxes: `WasmtimeRunner` with fuel and memory metering, and `HostSandboxRunner` with compiler error diagnostics.
  - Ratatui full-screen interactive cockpit with live code editor, syntax highlighting, diagnostic quiz widget, and animated Socratic Lighthouse.
  - SQLite persistence with WAL mode, schema migrations, and session management.
  - Multi-format session exports: Markdown, Asciinema v2 (`.cast`), and vector SVG badges.
  - Multi-channel distribution packaging: npm/npx binary launcher (`@nereuslabs/nereus`), Homebrew formula (`nereuslabs/tap/nereus`), and shell installer.

---

## [0.1.0] - 2026-11-02

### Added
- Architectural specification RFC, algebraic state enum drafts, domain models, and MIT open-source licensing.
- Deprecation and archival of legacy Python v1 prototype at `nereuslabs/nereus-v1`.
