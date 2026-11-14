//! Socratic Mentor Benchmark subcommand execution handler.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crate::args::BenchArgs;
use anyhow::Result;
use nereus_agents::SocraticBenchmark;
use nereus_core::traits::LLMClient;
use nereus_llm::{AnthropicClient, OpenRouterClient, StubLLMClient};
use std::sync::Arc;

/// Executes the automated Socratic mentor benchmark across the canonical problem suite.
pub async fn run_benchmark_command(args: &BenchArgs) -> Result<()> {
    println!("🧪 Running Nereus Socratic Mentor Benchmark Suite...");
    println!("🤖 Target Model / Provider: {}", args.model);

    let llm: Arc<dyn LLMClient> = if args.offline || args.model.to_lowercase() == "stub" {
        println!("🛡️  Execution Mode: Hermetic Offline Stub (Deterministic, Zero API Tokens)\n");
        Arc::new(StubLLMClient::new())
    } else if args.model.to_lowercase().contains("openrouter") {
        let key = std::env::var("OPENROUTER_API_KEY")
            .or_else(|_| std::env::var("NEREUS_API_KEY"))
            .unwrap_or_else(|_| "mock-key".to_string());
        println!("🌐 Execution Mode: Live OpenRouter Inference\n");
        Arc::new(OpenRouterClient::new(key, Some(args.model.clone())))
    } else if args.model.to_lowercase().contains("claude")
        || args.model.to_lowercase().contains("anthropic")
    {
        let key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "mock-key".to_string());
        println!("🌐 Execution Mode: Live Anthropic API Inference\n");
        Arc::new(AnthropicClient::new(key))
    } else {
        println!("⚠️  Unrecognized live provider, defaulting to Deterministic Stub client.\n");
        Arc::new(StubLLMClient::new())
    };

    let benchmark = SocraticBenchmark::new(llm, &args.model);
    let report = benchmark.run_suite().await;

    let output_str = if args.format.to_lowercase() == "json" {
        report.to_json()?
    } else {
        report.to_markdown()
    };

    if let Some(export_path) = &args.export {
        std::fs::write(export_path, &output_str)?;
        println!("📁 Benchmark report exported to: {}", export_path);
    }

    println!("{}", output_str);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_benchmark_command_offline_json() {
        let args = BenchArgs {
            model: "stub".to_string(),
            offline: true,
            format: "json".to_string(),
            export: None,
        };

        let res = run_benchmark_command(&args).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_run_benchmark_command_offline_markdown() {
        let temp_file =
            std::env::temp_dir().join(format!("bench_test_{}.md", uuid::Uuid::new_v4()));
        let export_path = temp_file.to_string_lossy().to_string();

        let args = BenchArgs {
            model: "stub".to_string(),
            offline: true,
            format: "markdown".to_string(),
            export: Some(export_path.clone()),
        };

        let res = run_benchmark_command(&args).await;
        assert!(res.is_ok());
        assert!(std::path::Path::new(&export_path).exists());

        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(content.contains("# 🌊 Nereus Socratic Mentor Benchmark Report"));

        let _ = std::fs::remove_file(export_path);
    }
}
