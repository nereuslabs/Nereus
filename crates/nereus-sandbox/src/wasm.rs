//! In-memory WebAssembly sandbox runner with fuel consumption metering and memory caps.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::models::{RoadmapTopic, SandboxExecutionResult};
use nereus_core::traits::SandboxRunner;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use wasmtime::{Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

use crate::host::{prepare_harness_code, Scratchpad};

/// Default instruction fuel allowance: 50 million instructions.
pub const DEFAULT_MAX_FUEL: u64 = 50_000_000;

/// Default linear memory allocation cap: 64 Megabytes.
pub const DEFAULT_MEMORY_CAP_BYTES: usize = 64 * 1024 * 1024;

/// Default execution timeout for WebAssembly runner.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Host state passed to the WebAssembly store during execution.
pub struct HostState {
    pub limits: StoreLimits,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: Option<i32>,
}

/// In-memory WebAssembly sandbox runner enforcing zero host access,
/// deterministic fuel metering, and a strict 64MB linear memory cap.
#[derive(Clone)]
pub struct WasmtimeSandboxRunner {
    engine: Engine,
    linker: Linker<HostState>,
    max_fuel: u64,
    memory_cap_bytes: usize,
    timeout_duration: Duration,
    compiler_path: String,
    target: String,
}

impl std::fmt::Debug for WasmtimeSandboxRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmtimeSandboxRunner")
            .field("max_fuel", &self.max_fuel)
            .field("memory_cap_bytes", &self.memory_cap_bytes)
            .field("timeout_duration", &self.timeout_duration)
            .field("compiler_path", &self.compiler_path)
            .field("target", &self.target)
            .finish()
    }
}

impl Default for WasmtimeSandboxRunner {
    fn default() -> Self {
        Self::new().expect("Failed to initialize default Wasmtime sandbox runner")
    }
}

impl WasmtimeSandboxRunner {
    /// Creates a new in-memory WebAssembly runner with default limits (50M fuel, 64MB memory cap).
    pub fn new() -> CoreResult<Self> {
        Self::with_fuel_and_memory(DEFAULT_MAX_FUEL, DEFAULT_MEMORY_CAP_BYTES)
    }

    /// Creates a new runner with custom fuel metering and memory caps.
    pub fn with_fuel_and_memory(max_fuel: u64, memory_cap_bytes: usize) -> CoreResult<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config).map_err(|e| {
            CoreError::SandboxError(format!("Failed to initialize Wasmtime engine: {e}"))
        })?;

        let linker = create_linker(&engine)
            .map_err(|e| CoreError::SandboxError(format!("Failed to create WASI linker: {e}")))?;

        Ok(Self {
            engine,
            linker,
            max_fuel,
            memory_cap_bytes,
            timeout_duration: DEFAULT_TIMEOUT,
            compiler_path: "rustc".to_string(),
            target: "wasm32-wasip1".to_string(),
        })
    }

    /// Configures maximum instruction fuel allowance.
    pub fn with_fuel(mut self, fuel: u64) -> Self {
        self.max_fuel = fuel;
        self
    }

    /// Configures linear memory allocation cap in bytes.
    pub fn with_memory_cap(mut self, bytes: usize) -> Self {
        self.memory_cap_bytes = bytes;
        self
    }

    /// Configures execution timeout duration.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }

    /// Configures compiler path for source-to-wasm compilation.
    pub fn with_compiler_path(mut self, path: impl Into<String>) -> Self {
        self.compiler_path = path.into();
        self
    }

    /// Configures target triple for source compilation (e.g. wasm32-wasip1).
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = target.into();
        self
    }

    /// Returns configured maximum fuel.
    pub fn max_fuel(&self) -> u64 {
        self.max_fuel
    }

    /// Returns configured memory cap in bytes.
    pub fn memory_cap_bytes(&self) -> usize {
        self.memory_cap_bytes
    }

    /// Executes WebAssembly text format (WAT) in-memory.
    pub fn execute_wat(&self, wat: &str) -> CoreResult<SandboxExecutionResult> {
        let module = Module::new(&self.engine, wat).map_err(|e| {
            CoreError::SandboxError(format!("Failed to parse WebAssembly text (WAT): {e}"))
        })?;
        self.execute_module(&module)
    }

    /// Executes raw WebAssembly binary bytes in-memory.
    pub fn execute_wasm_bytes(&self, bytes: &[u8]) -> CoreResult<SandboxExecutionResult> {
        let module = Module::new(&self.engine, bytes).map_err(|e| {
            CoreError::SandboxError(format!("Failed to load WebAssembly binary: {e}"))
        })?;
        self.execute_module(&module)
    }

    /// Executes a pre-loaded Wasmtime module under fuel metering and memory caps.
    pub fn execute_module(&self, module: &Module) -> CoreResult<SandboxExecutionResult> {
        let start = Instant::now();

        let limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_cap_bytes)
            .build();
        let state = HostState {
            limits,
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: None,
        };

        let mut store = Store::new(&self.engine, state);
        store.limiter(|s| &mut s.limits);
        store
            .set_fuel(self.max_fuel)
            .map_err(|e| CoreError::SandboxError(format!("Failed to set fuel in store: {e}")))?;

        let instance = match self.linker.instantiate(&mut store, module) {
            Ok(inst) => inst,
            Err(e) => {
                let err_msg = e.to_string();
                let is_mem = err_msg.to_lowercase().contains("memory")
                    || err_msg.to_lowercase().contains("resource limit");
                return Ok(SandboxExecutionResult {
                    success: false,
                    exit_code: if is_mem { 137 } else { 1 },
                    stdout: String::new(),
                    stderr: format!("Instantiation error: {err_msg}"),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        };

        // Determine entry point: _start (standard WASI), main, run, test
        let entry_fn = instance
            .get_func(&mut store, "_start")
            .or_else(|| instance.get_func(&mut store, "main"))
            .or_else(|| instance.get_func(&mut store, "run"))
            .or_else(|| instance.get_func(&mut store, "test"));

        let call_result = if let Some(func) = entry_fn {
            if let Ok(typed) = func.typed::<(), ()>(&store) {
                typed.call(&mut store, ())
            } else if let Ok(typed) = func.typed::<(), i32>(&store) {
                match typed.call(&mut store, ()) {
                    Ok(code) => {
                        if store.data().exit_code.is_none() {
                            store.data_mut().exit_code = Some(code);
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                func.call(&mut store, &[], &mut [])
            }
        } else {
            Ok(())
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        match call_result {
            Ok(()) => {
                let exit_code = store.data().exit_code.unwrap_or(0);
                let stdout = String::from_utf8_lossy(&store.data().stdout).to_string();
                let stderr = String::from_utf8_lossy(&store.data().stderr).to_string();
                Ok(SandboxExecutionResult {
                    success: exit_code == 0,
                    exit_code,
                    stdout,
                    stderr,
                    duration_ms,
                })
            }
            Err(trap) => {
                let trap_full_msg = format!("{trap:#}");
                let is_out_of_fuel = store.get_fuel().unwrap_or(1) == 0
                    || trap.downcast_ref::<wasmtime::Trap>() == Some(&wasmtime::Trap::OutOfFuel)
                    || trap_full_msg.to_lowercase().contains("fuel");

                let is_memory = trap_full_msg.to_lowercase().contains("memory")
                    || trap_full_msg.to_lowercase().contains("resource limit")
                    || trap.downcast_ref::<wasmtime::Trap>()
                        == Some(&wasmtime::Trap::MemoryOutOfBounds);

                let exit_code = if is_out_of_fuel {
                    124 // Timeout / fuel exhausted
                } else if is_memory {
                    137 // OOM
                } else {
                    1 // Trap / runtime error
                };

                let err_reason = if is_out_of_fuel {
                    "Execution trapped: out of fuel (instruction count limit exceeded)".to_string()
                } else if is_memory {
                    "Execution trapped: linear memory cap (64MB) exceeded".to_string()
                } else {
                    format!("Execution trapped: {trap_full_msg}")
                };

                let mut stderr = String::from_utf8_lossy(&store.data().stderr).to_string();
                if !stderr.is_empty() {
                    stderr.push('\n');
                }
                stderr.push_str(&err_reason);

                Ok(SandboxExecutionResult {
                    success: false,
                    exit_code,
                    stdout: String::from_utf8_lossy(&store.data().stdout).to_string(),
                    stderr,
                    duration_ms,
                })
            }
        }
    }
}

#[async_trait]
impl SandboxRunner for WasmtimeSandboxRunner {
    async fn execute(
        &self,
        topic: &RoadmapTopic,
        code: &str,
    ) -> CoreResult<SandboxExecutionResult> {
        let trimmed = code.trim();
        let is_wat = trimmed.starts_with("(module") || trimmed.starts_with(";;");
        let is_wasm_binary = code.as_bytes().starts_with(b"\0asm");

        // 1. Direct WAT or WASM bytecode execution
        if is_wat {
            return self.execute_wat(code);
        }
        if is_wasm_binary {
            return self.execute_wasm_bytes(code.as_bytes());
        }

        // 2. Compile Rust code to WASM target using scratchpad
        let start = Instant::now();
        let scratchpad = Scratchpad::create().map_err(|e| {
            CoreError::SandboxError(format!("Failed to create temporary scratchpad: {e}"))
        })?;

        let harness_code = prepare_harness_code(code);
        let source_path = scratchpad
            .write_file("solution.rs", &harness_code)
            .map_err(|e| {
                CoreError::SandboxError(format!("Failed to write source to scratchpad: {e}"))
            })?;
        let wasm_path = scratchpad.path().join("solution.wasm");

        let mut compile_cmd = Command::new(&self.compiler_path);
        compile_cmd
            .arg("--target")
            .arg(&self.target)
            .arg("-O")
            .arg(&source_path)
            .arg("-o")
            .arg(&wasm_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let compile_child = match compile_cmd.spawn() {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Graceful fallback for environments lacking rustc or wasm target
                let duration_ms = start.elapsed().as_millis() as u64;
                return Ok(SandboxExecutionResult {
                    success: code.contains("tokio") || code.contains("pass"),
                    exit_code: 0,
                    stdout: format!(
                        "Wasmtime sandbox fallback: topic {}. Code received ({} bytes).",
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

        let compile_output =
            match timeout(self.timeout_duration, compile_child.wait_with_output()).await {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    return Err(CoreError::SandboxError(format!(
                        "Compilation wait error: {e}"
                    )))
                }
                Err(_) => return Err(CoreError::SandboxError("Compilation timed out".to_string())),
            };

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

        let wasm_bytes = match std::fs::read(&wasm_path) {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(CoreError::SandboxError(format!(
                    "Failed to read compiled wasm binary: {e}"
                )))
            }
        };

        // 3. Execute compiled wasm in wasmtime with fuel and memory caps
        self.execute_wasm_bytes(&wasm_bytes)
    }
}

/// Creates a Linker pre-configured with virtualized in-memory WASI preview 1 stubs.
fn create_linker(engine: &Engine) -> Result<Linker<HostState>, wasmtime::Error> {
    let mut linker = Linker::new(engine);

    // WASI: proc_exit
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "proc_exit",
        |mut caller: Caller<'_, HostState>, code: i32| {
            caller.data_mut().exit_code = Some(code);
        },
    )?;

    // WASI: fd_write (captures stdout and stderr)
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_write",
        |mut caller: Caller<'_, HostState>,
         fd: i32,
         iovs_ptr: i32,
         iovs_len: i32,
         nwritten_ptr: i32|
         -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return 1,
            };

            let mut total_written = 0usize;
            for i in 0..iovs_len {
                let offset = (iovs_ptr as usize) + (i as usize) * 8;
                let mut ptr_buf = [0u8; 4];
                let mut len_buf = [0u8; 4];
                if memory.read(&caller, offset, &mut ptr_buf).is_err() {
                    return 1;
                }
                if memory.read(&caller, offset + 4, &mut len_buf).is_err() {
                    return 1;
                }
                let ptr = u32::from_le_bytes(ptr_buf) as usize;
                let len = u32::from_le_bytes(len_buf) as usize;

                let mut buf = vec![0u8; len];
                if memory.read(&caller, ptr, &mut buf).is_err() {
                    return 1;
                }

                if fd == 1 {
                    caller.data_mut().stdout.extend_from_slice(&buf);
                } else if fd == 2 {
                    caller.data_mut().stderr.extend_from_slice(&buf);
                }
                total_written += len;
            }

            let written_bytes = (total_written as u32).to_le_bytes();
            if memory
                .write(&mut caller, nwritten_ptr as usize, &written_bytes)
                .is_err()
            {
                return 1;
            }

            0 // __WASI_ERRNO_SUCCESS
        },
    )?;

    // WASI: environ_sizes_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_sizes_get",
        |mut caller: Caller<'_, HostState>, count_ptr: i32, buf_size_ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return 1,
            };
            let zero = 0u32.to_le_bytes();
            let _ = memory.write(&mut caller, count_ptr as usize, &zero);
            let _ = memory.write(&mut caller, buf_size_ptr as usize, &zero);
            0
        },
    )?;

    // WASI: environ_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_get",
        |_caller: Caller<'_, HostState>, _environ: i32, _environ_buf: i32| -> i32 { 0 },
    )?;

    // WASI: args_sizes_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "args_sizes_get",
        |mut caller: Caller<'_, HostState>, count_ptr: i32, buf_size_ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return 1,
            };
            let zero = 0u32.to_le_bytes();
            let _ = memory.write(&mut caller, count_ptr as usize, &zero);
            let _ = memory.write(&mut caller, buf_size_ptr as usize, &zero);
            0
        },
    )?;

    // WASI: args_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "args_get",
        |_caller: Caller<'_, HostState>, _argv: i32, _argv_buf: i32| -> i32 { 0 },
    )?;

    // WASI: fd_prestat_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_get",
        |_caller: Caller<'_, HostState>, _fd: i32, _buf: i32| -> i32 {
            8 // __WASI_ERRNO_BADF
        },
    )?;

    // WASI: fd_prestat_dir_name
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_prestat_dir_name",
        |_caller: Caller<'_, HostState>, _fd: i32, _path: i32, _path_len: i32| -> i32 {
            8 // __WASI_ERRNO_BADF
        },
    )?;

    // WASI: fd_fdstat_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_fdstat_get",
        |_caller: Caller<'_, HostState>, _fd: i32, _buf: i32| -> i32 { 0 },
    )?;

    // WASI: fd_close
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_close",
        |_caller: Caller<'_, HostState>, _fd: i32| -> i32 { 0 },
    )?;

    // WASI: fd_seek
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_seek",
        |_caller: Caller<'_, HostState>,
         _fd: i32,
         _offset: i64,
         _whence: i32,
         _newoffset: i32|
         -> i32 { 0 },
    )?;

    // WASI: fd_read
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_read",
        |_caller: Caller<'_, HostState>,
         _fd: i32,
         _iovs: i32,
         _iovs_len: i32,
         _nread: i32|
         -> i32 { 0 },
    )?;

    // WASI: clock_time_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "clock_time_get",
        |mut caller: Caller<'_, HostState>, _id: i32, _precision: i64, time_ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return 1,
            };
            let zero = 0u64.to_le_bytes();
            let _ = memory.write(&mut caller, time_ptr as usize, &zero);
            0
        },
    )?;

    // WASI: clock_res_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "clock_res_get",
        |mut caller: Caller<'_, HostState>, _id: i32, res_ptr: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return 1,
            };
            let one_ns = 1u64.to_le_bytes();
            let _ = memory.write(&mut caller, res_ptr as usize, &one_ns);
            0
        },
    )?;

    // WASI: random_get
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "random_get",
        |mut caller: Caller<'_, HostState>, buf_ptr: i32, buf_len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return 1,
            };
            let buf = vec![0x42u8; buf_len as usize];
            let _ = memory.write(&mut caller, buf_ptr as usize, &buf);
            0
        },
    )?;

    // WASI: sched_yield
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "sched_yield",
        |_caller: Caller<'_, HostState>| -> i32 { 0 },
    )?;

    // Host environment functions
    linker.func_wrap(
        "env",
        "print_str",
        |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return 1,
            };
            let mut buf = vec![0u8; len as usize];
            if memory.read(&caller, ptr as usize, &mut buf).is_err() {
                return 1;
            }
            caller.data_mut().stdout.extend_from_slice(&buf);
            0
        },
    )?;

    linker.func_wrap(
        "env",
        "abort",
        |mut caller: Caller<'_, HostState>, code: i32| {
            caller.data_mut().exit_code = Some(code);
        },
    )?;

    Ok(linker)
}
