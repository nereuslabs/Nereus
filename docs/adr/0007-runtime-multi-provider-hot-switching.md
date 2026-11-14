# ADR 0007: Runtime Multi-Provider Hot-Switching Architecture

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-14

---

## Context & Problem Statement

Users operating an interactive CLI or terminal tutor work under varied environments:
1. Network connectivity fluctuates between online (OpenRouter, Anthropic) and offline (local development, travel).
2. API token quotas may exhaust mid-session without notice.
3. Execution security requirements vary: users may prefer fast, unrestricted local compiler testing (`HostSandboxRunner`) or strictly sandboxed, fuel-metered WebAssembly execution (`WasmtimeSandboxRunner`).

Restarting an interactive session to change CLI flags disrupts the learner's focus and requires recreating transient state.

## Decision Drivers

* Dynamic runtime reconfigurability without session destruction or process restarts.
* Type-safe dynamic dispatch over core traits (`LlmClient`, `SandboxRunner`).
* Clear, accessible keyboard shortcut in the Ratatui TUI cockpit (`Ctrl+P`, `F2`, `p`, or `:`).

## Considered Options

1. **Static compile-time feature flags (`--features wasmtime,openrouter`)**: Zero runtime overhead, but requires separate binary builds and forbids live switching.
2. **Configuration file reload on `SIGHUP`**: Standard UNIX daemon pattern, but poor UX for terminal-interactive users.
3. **ModuleRegistry dynamic container with TUI modal dialog**: Tier-1 dynamic registry holding registered providers behind trait objects (`Arc<dyn LlmClient>`, `Arc<dyn SandboxRunner>`), selectable via an in-cockpit overlay.

## Decision Outcome

Chosen option: **`ModuleRegistry` dynamic container with TUI modal dialog** in `nereus-plugin` and `nereus-tui`.

```rust
pub struct ModuleRegistry {
    llm_providers: HashMap<String, Arc<dyn LlmClient>>,
    sandbox_runners: HashMap<String, Arc<dyn SandboxRunner>>,
    active_llm: String,
    active_sandbox: String,
}
```

In the TUI, pressing `Ctrl+P` (or `F2`, `p`, `:`) opens `ProviderModalState`, allowing keyboard navigation (`Up`/`Down`, `Left`/`Right`, `Tab`, `Enter`) to switch active inference and execution backends instantly.

### Consequences

* **Positive**: Users switch between `OpenRouter`, `Anthropic`, and `Stub` or between `Host` and `Wasmtime` on the fly.
* **Positive**: Session state, editor buffer, and learning roadmap remain preserved during backend swaps.
* **Positive**: Seamless fallback to `StubLLMClient` when offline.
* **Negative**: Dynamic dispatch via `Arc<dyn Trait>` introduces microscopic vtable pointer indirections (nanoseconds), wholly negligible compared to terminal I/O and LLM network latencies.
