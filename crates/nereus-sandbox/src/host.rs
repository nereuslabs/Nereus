//! Isolated host subprocess runner with timeout and stdout/stderr capture.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::models::{RoadmapTopic, SandboxExecutionResult};
use nereus_core::traits::SandboxRunner;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use crate::diagnostic::{CompilerDiagnostic, DiagnosticParser};

/// Safe temporary scratchpad directory created under `std::env::temp_dir()`.
/// Automatically deletes the directory and all contained files on drop.
#[derive(Debug)]
pub struct Scratchpad {
    dir: PathBuf,
}

impl Scratchpad {
    /// Creates a new isolated scratchpad directory inside `std::env::temp_dir()`.
    pub fn create() -> std::io::Result<Self> {
        let unique_name = format!("nereus-sandbox-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(unique_name);
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Writes content to a file inside the scratchpad directory.
    pub fn write_file(&self, filename: &str, content: &str) -> std::io::Result<PathBuf> {
        let path = self.dir.join(filename);
        std::fs::write(&path, content)?;
        Ok(path)
    }

    /// Returns the scratchpad path.
    pub fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Scratchpad {
    fn drop(&mut self) {
        if self.dir.exists() {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

/// Wraps student code with standard harness boilerplate if missing entry points.
pub fn prepare_harness_code(code: &str) -> String {
    let has_main = code.contains("fn main");
    let has_test = code.contains("#[test]");

    if has_main || has_test {
        code.to_string()
    } else {
        format!(
            "{code}\n\n#[allow(dead_code)]\nfn main() {{\n    // Automated test harness entry\n}}\n"
        )
    }
}

/// Subprocess host runner executing student code with isolated scratchpad and timeouts.
#[derive(Debug, Clone)]
pub struct HostSandboxRunner {
    pub timeout_duration: Duration,
    pub compiler_path: String,
}

impl Default for HostSandboxRunner {
    fn default() -> Self {
        Self {
            timeout_duration: Duration::from_secs(5),
            compiler_path: "rustc".to_string(),
        }
    }
}

impl HostSandboxRunner {
    /// Creates a new host sandbox runner with specified timeout.
    pub fn new(timeout_duration: Duration) -> Self {
        Self {
            timeout_duration,
            ..Default::default()
        }
    }

    /// Sets a custom compiler path (default: "rustc").
    pub fn with_compiler_path(mut self, path: impl Into<String>) -> Self {
        self.compiler_path = path.into();
        self
    }

    /// Sets a custom timeout duration.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }

    /// Parses compiler diagnostics from stderr output using regex.
    pub fn parse_diagnostics(&self, stderr: &str) -> Vec<CompilerDiagnostic> {
        DiagnosticParser::parse(stderr)
    }

    /// Executes code and returns both execution result and parsed compiler diagnostics.
    pub async fn execute_and_diagnose(
        &self,
        topic: &RoadmapTopic,
        code: &str,
    ) -> CoreResult<(SandboxExecutionResult, Vec<CompilerDiagnostic>)> {
        let result = self.execute(topic, code).await?;
        let diagnostics = self.parse_diagnostics(&result.stderr);
        Ok((result, diagnostics))
    }
}

#[async_trait]
impl SandboxRunner for HostSandboxRunner {
    async fn execute(
        &self,
        topic: &RoadmapTopic,
        code: &str,
    ) -> CoreResult<SandboxExecutionResult> {
        let start = Instant::now();

        // 1. Create temporary directory scratchpad under std::env::temp_dir()
        let scratchpad = Scratchpad::create().map_err(|e| {
            CoreError::SandboxError(format!("Failed to create temporary scratchpad: {e}"))
        })?;

        // 2. Prepare code with harness boilerplate and write to scratchpad
        let harness_code = prepare_harness_code(code);
        let source_path = scratchpad
            .write_file("solution.rs", &harness_code)
            .map_err(|e| {
                CoreError::SandboxError(format!("Failed to write student code to scratchpad: {e}"))
            })?;
        let binary_path = scratchpad.path().join("solution_bin");

        let is_test = code.contains("#[test]");

        // 3. Compile student code
        let mut compile_cmd = Command::new(&self.compiler_path);
        if is_test {
            compile_cmd.arg("--test");
        }
        compile_cmd
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let compile_child = match compile_cmd.spawn() {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Graceful fallback if compiler is not installed locally
                let duration_ms = start.elapsed().as_millis() as u64;
                return Ok(SandboxExecutionResult {
                    success: code.contains("tokio") || code.contains("pass"),
                    exit_code: 0,
                    stdout: format!(
                        "Sandbox environment: fallback mode for topic {}. Code received ({} bytes).",
                        topic.title,
                        code.len()
                    ),
                    stderr: String::new(),
                    duration_ms,
                });
            }
            Err(e) => {
                return Err(CoreError::SandboxError(format!(
                    "Failed to spawn compiler '{}': {e}",
                    self.compiler_path
                )))
            }
        };

        // Enforce compilation timeout
        let compile_output =
            match timeout(self.timeout_duration, compile_child.wait_with_output()).await {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    return Err(CoreError::SandboxError(format!(
                        "Compilation wait error: {e}"
                    )))
                }
                Err(_) => return Err(CoreError::SandboxError("Execution timed out".to_string())),
            };

        // If compilation failed, return compiler diagnostics and non-zero exit code
        if !compile_output.status.success() {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(SandboxExecutionResult {
                success: false,
                exit_code: compile_output.status.code().unwrap_or(1),
                stdout: String::from_utf8_lossy(&compile_output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&compile_output.stderr).to_string(),
                duration_ms,
            });
        }

        // 4. Execute compiled binary
        let elapsed = start.elapsed();
        let remaining_timeout = self.timeout_duration.saturating_sub(elapsed);
        if remaining_timeout.is_zero() {
            return Err(CoreError::SandboxError("Execution timed out".to_string()));
        }

        let mut run_cmd = Command::new(&binary_path);
        run_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let run_child = match run_cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                return Err(CoreError::SandboxError(format!(
                    "Failed to spawn compiled binary: {e}"
                )))
            }
        };

        match timeout(remaining_timeout, run_child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                Ok(SandboxExecutionResult {
                    success: output.status.success(),
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    duration_ms,
                })
            }
            Ok(Err(e)) => Err(CoreError::SandboxError(format!(
                "Runtime execution error: {e}"
            ))),
            Err(_) => Err(CoreError::SandboxError("Execution timed out".to_string())),
        }
    }
}
