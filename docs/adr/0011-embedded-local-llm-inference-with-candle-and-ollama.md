# ADR 0011: Embedded Local LLM Inference with Candle and Ollama

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-14

---

## Context & Problem Statement

Nereus 2.0 MVP supports cloud-hosted LLMs (`OpenRouter`, `Anthropic Claude`) and deterministic offline testing (`StubLLMClient`). However, learners frequently require private, offline learning assistance without paid API keys, internet connectivity, or token latency.

## Decision Drivers

* Complete data privacy and offline autonomy.
* Low memory footprint and instant startup (<500ms).
* Hardware acceleration support (Metal on macOS, Vulkan/CUDA on Linux and Windows).
* Seamless integration into `LlmClient` trait without breaking changes.

## Considered Options

1. **Python Sidecar with `vLLM` or `transformers`**: Rejected due to high memory consumption (>4GB) and heavyweight Python runtime dependencies.
2. **`llama.cpp` via C++ bindings (`llama-cpp-rs`)**: High performance, but introduces complex C++ toolchain and CMake build dependencies that complicate cross-compilation.
3. **Pure Rust `candle` framework combined with local `Ollama` socket bridge**:
   - Pure Rust execution via Hugging Face's `candle` with quantized GGUF weights for in-process execution.
   - HTTP/socket bridge to existing local `Ollama` daemon (`localhost:11434`) when present.

## Decision Outcome

Chosen option: **Pure Rust `candle` framework combined with local `Ollama` socket bridge** in `nereus-llm`.

```rust
pub struct LocalCandleClient {
    model: Arc<candle_transformers::models::quantized_llama::ModelWeights>,
    tokenizer: tokenizers::Tokenizer,
    device: candle_core::Device,
}

pub struct OllamaClient {
    endpoint: String,
    model_name: String,
    http_client: reqwest::Client,
}
```

Both clients implement `nereus_core::traits::LLMClient`, allowing runtime selection via the existing `ModuleRegistry` and in-cockpit hot-switching dialog (`Ctrl+P`).

### Consequences

* **Positive**: Zero API key dependencies for learners; complete privacy.
* **Positive**: Fully native Rust compilation without C++ build dependencies via `candle`.
* **Positive**: Integrates cleanly with existing `LlmClient` async interface.
* **Negative**: In-process model execution requires ~1.5GB to 3GB RAM depending on quantized parameter size (1.5B vs 3B parameters).
