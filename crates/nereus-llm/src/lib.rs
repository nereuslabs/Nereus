//! Nereus LLM: Provider implementations, prompt templates, and testing stubs.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod openrouter;
pub mod prompts;
pub mod stub;

pub use openrouter::OpenRouterClient;
pub use prompts::*;
pub use stub::StubLLMClient;
