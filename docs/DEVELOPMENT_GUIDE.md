# 🛠️ Nereus 2.0 Development & Engineering Guide

## 1. Prerequisites & Environment Setup

Nereus 2.0 is built exclusively in **Rust** (2021 Edition).

### System Requirements
* **Minimum Supported Rust Version (MSRV)**: Rust 1.78.0 or newer.
* **Operating Systems**: macOS (Apple Silicon & Intel), Linux (x86_64 & aarch64), Windows 11.
* **Git**: 2.30+ (with support for `git worktree`).

### Toolchain Installation
Install the official Rust toolchain via `rustup`:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Verify your installation:
```bash
rustc --version
cargo --version
```

Add required components:
```bash
rustup component add rustfmt
rustup component add clippy
rustup target add wasm32-wasi  # For WebAssembly sandbox development
```

---

## 2. Repository Structure & Building

Clone the repository and inspect the workspace:
```bash
git clone https://github.com/nereuslabs/nereus.git
cd nereus
```

### Build Commands
```bash
# Debug build (fast compile, development profile)
cargo build

# Optimized release binary
cargo build --release

# The compiled binary is located at:
./target/release/nereus --help
```

---

## 3. Offline-First Testing Discipline

### The Non-Negotiable Testing Rule
> **Zero API keys and zero network connectivity are required to run 100% of the unit, integration, and UI tests.**

Development and CI pipelines must remain hermetic and deterministic. Live LLM calls are strictly gated behind explicit opt-in environment variables and run only in scheduled or manual benchmark suites.

### Running Test Suites
```bash
# Run all unit and hermetic integration tests across all workspace crates
cargo test

# Run tests for a specific crate (e.g. nereus-agents)
cargo test -p nereus-agents

# Run tests with backtrace on failure
RUST_BACKTRACE=1 cargo test -- --nocapture

# Benchmark execution speeds
cargo bench
```

### Testing with the Deterministic Stub Client
In test suites, instantiate `StubLLMClient` to mock LLM responses:
```rust
#[tokio::test]
async fn test_examiner_retry_loop() {
    let stub_llm = Arc::new(StubLLMClient::new());
    let mock_sandbox = Arc::new(MockSandboxRunner::failing_compiler());
    
    let examiner = ExaminerAgent::new(stub_llm, mock_sandbox);
    let assessment = examiner.evaluate_submission(&topic, "pub fn broken() {}").await.unwrap();
    
    assert_eq!(assessment.verdict, Verdict::Retry);
    assert!(assessment.feedback.contains("compiler error"));
}
```

---

## 4. Coding Standards & Quality Gates

All pull requests and commits are verified by automated GitHub Actions CI. Code that fails any of these quality gates will be rejected:

### 1. Code Formatting (`rustfmt`)
All code must adhere to the official Rust formatting rules:
```bash
# Check formatting without modifying files (CI check)
cargo fmt --all -- --check

# Automatically format all workspace files
cargo fmt --all
```

### 2. Linting & Idiomatic Checks (`clippy`)
Clippy warnings are treated as hard errors in CI:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Common Clippy invariants enforced:
* Prefer explicit pattern matching over `unwrap()`.
* Document all public crate APIs (`#![warn(missing_docs)]`).
* Use `anyhow` for top-level application errors and `thiserror` for library domain errors.

---

## 5. Git Worktree Management for Modules

Nereus uses Git Worktrees to isolate development across modular crates. This allows multiple features to be developed simultaneously in independent working folders.

### Creating an Isolated Worktree
```bash
# Create a worktree under worktrees/ for a specific crate feature
git worktree add ../worktrees/sandbox-wasm -b feat/sandbox-wasmtime develop

# Navigate to the isolated worktree
cd ../worktrees/sandbox-wasm
```

### Best Practices for Worktree Isolation
1. **Scope Commits to One Crate**: Keep changes strictly localized to the target crate under `crates/<crate-name>/`.
2. **Never Edit Common Types Without Sync**: If modifying `crates/nereus-core/`, ensure downstream crates are updated and verified across all active branches before opening a PR.
3. **Clean Up When Merged**:
   ```bash
   cd /Users/yan/Pets/Nereus
   git worktree remove ../worktrees/sandbox-wasm
   ```

---

## 6. Conventional Commits Specification

All commit messages in the repository and across all pull requests must strictly follow the **[Conventional Commits 1.0.0](https://www.conventionalcommits.org/)** specification:

```text
<type>(<optional scope>): <description>

[optional body]

[optional footer(s)]
```

### Allowed Types
* `feat`: A new feature, agent, or subsystem capability.
* `fix`: A bug fix or state machine transition correction.
* `docs`: Documentation updates, architecture specs, or wiki sync.
* `refactor`: Code restructuring without functional or behavioural changes.
* `test`: Adding or correcting offline tests and benchmarks.
* `chore`: Build system, CI workflow, or dependency bumps.

### Example Commits
```text
feat(sandbox): implement Wasmtime fuel consumption and memory bounds

fix(fsm): prevent duplicate topic advancement on examiner retry

docs(architecture): add Mermaid sequence diagram for tutoring loop
```

---

## 7. Environment Variables Reference

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `NEREUS_OFFLINE` | `false` | When set to `true` or `1`, forces all agents to use the deterministic `StubLLMClient`. |
| `NEREUS_API_KEY` | *(None)* | OpenRouter or Anthropic API key for live inference. |
| `NEREUS_MODEL` | `openrouter/auto` | Target model slug for live LLM completions. |
| `NEREUS_DB_PATH` | `~/.nereus/nereus.db`| File path to the SQLite session storage database. |
| `NEREUS_TIMEOUT` | `5` | Maximum execution timeout (in seconds) for sandbox runs. |
| `RUST_LOG` | `info` | Log verbosity filter (`trace`, `debug`, `info`, `warn`, `error`). |

---

<div align="center">
  <sub>Nereus Labs Development Guide • v2.0 • MIT License</sub>
</div>
