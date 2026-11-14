# ADR 0003: Wasmtime & Host Compiler Dual Execution Sandbox

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-10

---

## Context & Problem Statement

To evaluate student code objectively without solution leaking or naive LLM prompt guessing, Nereus must compile and execute student solutions. However:
1. Untrusted student code could trigger infinite loops, fork bombs, or filesystem abuse.
2. Advanced Rust concepts (e.g., multithreading, async channels with `tokio`) require real compiler and runtime feedback.

## Decision Drivers

* Strict isolation and denial-of-service prevention (fuel-metered execution).
* High-fidelity compiler diagnostics (parsing exact error codes like `E0382`, `E0597`).
* Ability to test full Rust language semantics including crates.

## Considered Options

1. Pure Docker / OCI container sandboxing (high daemon overhead, not available on all developer machines).
2. Pure Wasmtime in-memory runner (memory-safe, fuel-metered, but lacks direct access to native compiler crates without pre-compiling wasm binaries).
3. Dual Hybrid Sandbox: In-memory `WasmtimeSandboxRunner` + Isolated Subprocess `HostSandboxRunner`.

## Decision Outcome

Chosen option: **Dual Execution Sandbox**:
1. `WasmtimeSandboxRunner`: In-memory WebAssembly engine with strict fuel metering (10,000,000 fuel limit) and memory limits (64MB) for hermetic verification.
2. `HostSandboxRunner`: Process-isolated scratchpad directory with `rustc` compiler execution, regex-based `DiagnosticParser`, and execution timeouts for native compilation tasks.

### Consequences

* **Positive**: Fast, memory-safe execution with zero daemon requirements.
* **Positive**: Exact compiler diagnostics are passed directly to `TutorAgent` and `ExaminerAgent`.
* **Negative**: Requires host machine to have a functional `rustc` compiler when executing native code challenges.
