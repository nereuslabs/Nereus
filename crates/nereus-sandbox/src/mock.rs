//! Deterministic mock sandbox runner for hermetic test suites.

use async_trait::async_trait;
use nereus_core::error::CoreResult;
use nereus_core::models::{RoadmapTopic, SandboxExecutionResult};
use nereus_core::traits::SandboxRunner;

#[derive(Debug, Default, Clone)]
pub struct MockSandboxRunner {
    pub force_pass: bool,
}

impl MockSandboxRunner {
    pub fn new(force_pass: bool) -> Self {
        Self { force_pass }
    }
}

#[async_trait]
impl SandboxRunner for MockSandboxRunner {
    async fn execute(&self, topic: &RoadmapTopic, code: &str) -> CoreResult<SandboxExecutionResult> {
        let is_pass = self.force_pass || code.to_lowercase().contains("pass") || code.contains("tokio");

        if is_pass {
            Ok(SandboxExecutionResult {
                success: true,
                exit_code: 0,
                stdout: format!("Running test suite for {}...\ntest result: ok. 4 passed; 0 failed.", topic.title),
                stderr: String::new(),
                duration_ms: 12,
            })
        } else {
            Ok(SandboxExecutionResult {
                success: false,
                exit_code: 101,
                stdout: format!("Compiling challenge for {}...\n", topic.title),
                stderr: "error[E0382]: use of moved value: `tx`\n  --> src/main.rs:12:9\n   |\n10 |     let tx2 = tx;\n   |               -- value moved here\n12 |     tx.send(42).await;\n   |     ^^ value used here after move".to_string(),
                duration_ms: 15,
            })
        }
    }
}
