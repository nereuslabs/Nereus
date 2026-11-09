//! Deterministic mock sandbox runner for hermetic test suites and zero-cost simulation.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::models::{RoadmapTopic, SandboxExecutionResult};
use nereus_core::traits::SandboxRunner;

/// Simulation scenarios supported by `MockSandboxRunner`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MockScenario {
    /// Simulates successful test execution with all assertions passing.
    Success,
    /// Simulates compiler failure with specified diagnostic error output.
    CompilerError(String),
    /// Simulates runtime test assertion failure / panic.
    AssertionFailed,
    /// Simulates hard timeout error during execution.
    Timeout,
    /// Evaluates code dynamically based on keywords (e.g. "pass", "tokio").
    #[default]
    Dynamic,
}

/// Mock sandbox runner providing deterministic outcomes for hermetic tests.
#[derive(Debug, Clone)]
pub struct MockSandboxRunner {
    pub force_pass: bool,
    pub scenario: MockScenario,
    pub simulated_duration_ms: u64,
}

impl Default for MockSandboxRunner {
    fn default() -> Self {
        Self::new(false)
    }
}

impl MockSandboxRunner {
    /// Creates a new mock runner with optional forced pass flag.
    pub fn new(force_pass: bool) -> Self {
        Self {
            force_pass,
            scenario: if force_pass {
                MockScenario::Success
            } else {
                MockScenario::Dynamic
            },
            simulated_duration_ms: 12,
        }
    }

    /// Creates a mock runner that always succeeds.
    pub fn success() -> Self {
        Self {
            force_pass: true,
            scenario: MockScenario::Success,
            simulated_duration_ms: 10,
        }
    }

    /// Alias for `success()` matching development documentation.
    pub fn passing() -> Self {
        Self::success()
    }

    /// Creates a mock runner that simulates compiler error with custom stderr message.
    pub fn compiler_error(msg: impl Into<String>) -> Self {
        Self {
            force_pass: false,
            scenario: MockScenario::CompilerError(msg.into()),
            simulated_duration_ms: 15,
        }
    }

    /// Creates a mock runner simulating standard compiler borrow check error.
    pub fn failing_compiler() -> Self {
        Self::compiler_error(
            "error[E0382]: use of moved value: `tx`\n  --> src/main.rs:12:9\n   |\n10 |     let tx2 = tx;\n   |               -- value moved here\n12 |     tx.send(42).await;\n   |     ^^ value used here after move\n\nFor more information about this error, try `rustc --explain E0382`.",
        )
    }

    /// Creates a mock runner that simulates assertion failure / test panic.
    pub fn assertion_failed() -> Self {
        Self {
            force_pass: false,
            scenario: MockScenario::AssertionFailed,
            simulated_duration_ms: 20,
        }
    }

    /// Creates a mock runner that simulates timeout error.
    pub fn timeout() -> Self {
        Self {
            force_pass: false,
            scenario: MockScenario::Timeout,
            simulated_duration_ms: 5000,
        }
    }

    /// Sets the active simulation scenario.
    pub fn with_scenario(mut self, scenario: MockScenario) -> Self {
        self.scenario = scenario;
        self
    }

    /// Sets custom simulated execution duration in milliseconds.
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.simulated_duration_ms = duration_ms;
        self
    }

    /// Dynamically switches simulation scenario.
    pub fn set_scenario(&mut self, scenario: MockScenario) {
        self.scenario = scenario;
    }
}

#[async_trait]
impl SandboxRunner for MockSandboxRunner {
    async fn execute(
        &self,
        topic: &RoadmapTopic,
        code: &str,
    ) -> CoreResult<SandboxExecutionResult> {
        match &self.scenario {
            MockScenario::Success => Ok(SandboxExecutionResult {
                success: true,
                exit_code: 0,
                stdout: format!(
                    "Running test suite for {}...\ntest result: ok. 4 passed; 0 failed.",
                    topic.title
                ),
                stderr: String::new(),
                duration_ms: self.simulated_duration_ms,
            }),
            MockScenario::CompilerError(msg) => {
                let err_msg = if msg.is_empty() {
                    "error[E0382]: use of moved value: `tx`\n  --> src/main.rs:12:9\n   |\n10 |     let tx2 = tx;\n   |               -- value moved here\n12 |     tx.send(42).await;\n   |     ^^ value used here after move"
                } else {
                    msg.as_str()
                };
                Ok(SandboxExecutionResult {
                    success: false,
                    exit_code: 101,
                    stdout: format!("Compiling challenge for {}...\n", topic.title),
                    stderr: err_msg.to_string(),
                    duration_ms: self.simulated_duration_ms,
                })
            }
            MockScenario::AssertionFailed => Ok(SandboxExecutionResult {
                success: false,
                exit_code: 101,
                stdout: format!(
                    "Running test suite for {}...\ntest tests::test_solution ... FAILED",
                    topic.title
                ),
                stderr: "thread 'tests::test_solution' panicked at src/lib.rs:25:9:\nassertion `left == right` failed\n  left: 4\n right: 5\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace".to_string(),
                duration_ms: self.simulated_duration_ms,
            }),
            MockScenario::Timeout => Err(CoreError::SandboxError("Execution timed out".to_string())),
            MockScenario::Dynamic => {
                let is_pass = self.force_pass
                    || code.to_lowercase().contains("pass")
                    || code.contains("tokio");

                if is_pass {
                    Ok(SandboxExecutionResult {
                        success: true,
                        exit_code: 0,
                        stdout: format!(
                            "Running test suite for {}...\ntest result: ok. 4 passed; 0 failed.",
                            topic.title
                        ),
                        stderr: String::new(),
                        duration_ms: self.simulated_duration_ms,
                    })
                } else {
                    Ok(SandboxExecutionResult {
                        success: false,
                        exit_code: 101,
                        stdout: format!("Compiling challenge for {}...\n", topic.title),
                        stderr: "error[E0382]: use of moved value: `tx`\n  --> src/main.rs:12:9\n   |\n10 |     let tx2 = tx;\n   |               -- value moved here\n12 |     tx.send(42).await;\n   |     ^^ value used here after move".to_string(),
                        duration_ms: self.simulated_duration_ms,
                    })
                }
            }
        }
    }
}
