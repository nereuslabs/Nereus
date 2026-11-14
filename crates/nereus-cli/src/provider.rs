//! Runtime dynamic provider and sandbox hot-switching coordinator for Nereus CLI.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use anyhow::Result;
use nereus_core::traits::{LLMClient, SandboxRunner};
use nereus_llm::{AnthropicClient, OpenRouterClient, StubLLMClient};
use nereus_plugin::ModuleRegistry;
use nereus_sandbox::{HostSandboxRunner, MockSandboxRunner, WasmtimeSandboxRunner};
use std::sync::Arc;
use tracing::info;

/// Hot-switches the active LLM provider in the module registry at runtime.
pub async fn switch_llm_provider(registry: &ModuleRegistry, provider_id: &str) -> Result<String> {
    let client: Arc<dyn LLMClient> = match provider_id.to_lowercase().as_str() {
        "stub" => Arc::new(StubLLMClient::new()),
        "openrouter" => {
            let key = std::env::var("OPENROUTER_API_KEY")
                .or_else(|_| std::env::var("NEREUS_API_KEY"))
                .unwrap_or_else(|_| "mock-openrouter-key".to_string());
            Arc::new(OpenRouterClient::new(
                key,
                Some("anthropic/claude-3.5-sonnet".to_string()),
            ))
        }
        "anthropic" => {
            let key = std::env::var("ANTHROPIC_API_KEY")
                .unwrap_or_else(|_| "mock-anthropic-key".to_string());
            Arc::new(AnthropicClient::new(key))
        }
        "ollama" => Arc::new(nereus_llm::OllamaClient::default()),
        "candle" | "local" => Arc::new(nereus_llm::LocalCandleClient::default()),
        other => anyhow::bail!("Unsupported LLM provider identifier: '{other}'"),
    };

    registry.hot_switch_llm(client).await;
    info!("Applied runtime Tier-1 hot-switch to LLM provider: {provider_id}");
    Ok(provider_id.to_string())
}

/// Hot-switches the active sandbox runner in the module registry at runtime.
pub async fn switch_sandbox_runner(registry: &ModuleRegistry, sandbox_id: &str) -> Result<String> {
    let runner: Arc<dyn SandboxRunner> = match sandbox_id.to_lowercase().as_str() {
        "mock" => Arc::new(MockSandboxRunner::new(false)),
        "host" => Arc::new(HostSandboxRunner::default()),
        "wasm" | "wasmtime" => Arc::new(WasmtimeSandboxRunner::new()?),
        other => anyhow::bail!("Unsupported Sandbox runner identifier: '{other}'"),
    };

    registry.hot_switch_sandbox(runner).await;
    info!("Applied runtime Tier-1 hot-switch to Sandbox runner: {sandbox_id}");
    Ok(sandbox_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nereus_storage::SqliteStorage;

    #[tokio::test]
    async fn test_hot_switch_llm_providers() {
        let storage = Arc::new(SqliteStorage::new(":memory:").unwrap());
        let registry = ModuleRegistry::new(
            Arc::new(StubLLMClient::new()),
            Arc::new(MockSandboxRunner::new(false)),
            storage,
        );

        // Switch to openrouter
        let res = switch_llm_provider(&registry, "openrouter").await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "openrouter");

        // Switch to anthropic
        let res = switch_llm_provider(&registry, "anthropic").await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "anthropic");

        // Switch to ollama
        let res = switch_llm_provider(&registry, "ollama").await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "ollama");

        // Switch to candle / local
        let res = switch_llm_provider(&registry, "candle").await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "candle");

        // Switch back to stub
        let res = switch_llm_provider(&registry, "stub").await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "stub");

        // Switch to invalid provider
        let err = switch_llm_provider(&registry, "unknown-provider").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_hot_switch_sandbox_runners() {
        let storage = Arc::new(SqliteStorage::new(":memory:").unwrap());
        let registry = ModuleRegistry::new(
            Arc::new(StubLLMClient::new()),
            Arc::new(MockSandboxRunner::new(false)),
            storage,
        );

        // Switch to host
        let res = switch_sandbox_runner(&registry, "host").await;
        assert!(res.is_ok());

        // Switch to wasmtime
        let res = switch_sandbox_runner(&registry, "wasmtime").await;
        assert!(res.is_ok());

        // Switch back to mock
        let res = switch_sandbox_runner(&registry, "mock").await;
        assert!(res.is_ok());

        // Switch to invalid runner
        let err = switch_sandbox_runner(&registry, "docker-sandbox").await;
        assert!(err.is_err());
    }
}
