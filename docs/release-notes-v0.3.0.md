# 🌊 Nereus v0.3.0 — Extensible Pedagogical Ecosystem, Local Inference & Multi-Agent Swarm

Nereus Labs is proud to announce the release of **Nereus v0.3.0**, our second major release. Following the launch of the v0.2.0 MVP, v0.3.0 establishes an extensible, local-first learning architecture: static code quality inspection, zero-cloud pure-Rust LLM inference, multi-agent Socratic guidance, interactive cockpit editor polish, full-text assessment search, and WebAssembly Component Model curriculum hosting.

---

## 🚀 Key Highlights & Architectural Innovations

### 1. 🔍 AST-Level Static Code Analysis & Anti-Pattern Inspector (`nereus-sandbox`)
- **Compile-Time Syntax Tree Traversal**: Implemented using `syn 2.0` with `full`, `visit`, and `span-locations`.
- **Proactive Anti-Pattern Detection**:
  - `RedundantClone`: Catches unnecessary deep copies where borrowed references (`&T`) suffice.
  - `DangerousUnwrap`: Highlights panic-prone `.unwrap()` calls in production-bound drills.
  - `UnsafeBlock`: Detects raw pointers and unencapsulated `unsafe` blocks without safety documentation.
  - `CStyleIndexLoop`: Flags manual index-based iteration (`for i in 0..v.len()`) to encourage idiomatic Rust iterators (`.iter()`).
- **Socratic Pedagogical Remediation**: Integrated directly with `ExaminerAgent` to formulate targeted hints without giving away solutions.

### 2. ⚡ In-Process Local LLM Inference Engine (`nereus-llm`)
- **Zero-Cloud, Pure-Rust Inference (`LocalCandleClient`)**: Pure-Rust in-process inference utilizing Hugging Face's `candle` tensor framework for completely offline environments.
- **Native Ollama Client (`OllamaClient`)**: Direct HTTP daemon integration communicating with local Ollama instances (`http://localhost:11434`) for local open-weights model inference.
- **Runtime Provider Switching**: In-cockpit hot-switching (`Ctrl+P` / `F2`) and CLI provider configuration (`nereus provider <name>`).

### 3. 📚 Librarian Agent: Semantic Retrieval & Compiler Citations (`nereus-agents`)
- **Offline Knowledge Corpus**: In-memory searchable knowledge base covering core Rust standard library primitives (`std::sync`, `std::cell`, `std::pin`, `std::marker`, `std::ops`), The Rustonomicon, and the Rust Reference.
- **Diagnostic Citation Engine**: Automatically maps compiler error codes (`E0382`, `E0597`, `E0502`, `E0106`, `E0277`) to targeted Socratic guidance and official book chapters.

### 4. 😈 Adversary Agent: Property-Based Educational Fuzzing (`nereus-agents`)
- **Automated Stress Test Generator**: Synthesizes pathological boundary inputs (`i64::MIN`, `i64::MAX`), empty collections, null bytes, and non-UTF-8 strings.
- **Concurrency & Unsafe Stress**: Generates scoped thread contention suites (`std::thread::scope`) and memory safety attack vectors to ensure true production readiness.

### 5. 🖥️ Ratatui Cockpit Editor Polish & UX (`nereus-tui`)
- **Unbounded Undo / Redo Ring**: Bi-directional historical state buffer (`Ctrl+Z`, `Ctrl+Y`) tracking non-linear text mutations.
- **Dynamic Line Numbering**: Left-aligned line numbers with compiler error gutter indicators.
- **In-Buffer Search Modal**: `Ctrl+F` modal dialog with real-time substring matching, match counts, and navigation.
- **Crossterm Mouse Integration**: Click-to-focus across all cockpit panes and smooth mousewheel scrolling.

### 6. 🔎 SQLite FTS5 Full-Text Search & Git Sync (`nereus-storage`, `nereus-cli`)
- **Full-Text Assessment Search**: Fast search across historical assessments and mentor feedback via `nereus search <query>` powered by SQLite FTS5.
- **Git Learning Journal Synchronization**: Bi-directional Git backup and version-controlled journal persistence via `nereus sync`.

### 7. 🧩 WebAssembly Component Model & WIT Host (`nereus-plugin`)
- **Canonical WIT Specification**: Interface definition `nereus:curriculum@0.3.0` enabling language-agnostic external curriculum authoring.
- **WasmCurriculumComponent Host**: In-memory binary loader verifying WASM headers (`\0asm`) and sidecar manifests.
- **Filesystem Discovery**: Automated plugin discovery in `~/.nereus/plugins/`.

---

## 📦 Multi-Channel Installation

### One-Command Instant Launch (via npx)
```bash
npx @nereuslabs/nereus@0.3.0
```

### Homebrew Tap (macOS & Linux)
```bash
brew tap nereuslabs/tap
brew install nereus
```

### POSIX Shell Installer (macOS & Linux)
```bash
curl -fsSL https://nereuslabs.github.io/install.sh | sh
```

### Cargo Source Installation
```bash
cargo install --git https://github.com/nereuslabs/nereus.git --tag v0.3.0 nereus-cli
```

---

## 🧪 Verification & Quality Metrics
- **Hermetic Offline Test Suite**: 100% passed across all 9 crates (100+ tests).
- **Quality Gates**: All 10/10 gates in `./scripts/verify_e2e.sh` verified.
- **Static Analysis**: `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings.
- **Formatting**: `cargo fmt --all --check` — 100% compliant.
- **Remote CI**: GitHub Actions automated pipeline passed on all supported platforms.
