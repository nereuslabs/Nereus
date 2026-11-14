//! Nereus LLM: Provider implementations, prompt templates, and testing stubs.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod anthropic;
pub mod candle;
pub mod client;
pub mod ollama;
pub mod openrouter;
pub mod prompts;
pub mod stub;

pub use anthropic::{AnthropicClient, AnthropicModel};
pub use candle::LocalCandleClient;
pub use client::{extract_json, parse_json, StructuredClient};
pub use ollama::{OllamaClient, DEFAULT_OLLAMA_ENDPOINT, DEFAULT_OLLAMA_MODEL};
pub use openrouter::OpenRouterClient;
pub use prompts::*;
pub use stub::StubLLMClient;
