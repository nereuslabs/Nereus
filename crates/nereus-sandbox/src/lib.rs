//! Nereus Sandbox: Execution-driven assessment environments.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod diagnostic;
pub mod host;
pub mod mock;
pub mod wasm;

pub use diagnostic::{CompilerDiagnostic, DiagnosticLevel, DiagnosticParser, DiagnosticSpan};
pub use host::{prepare_harness_code, HostSandboxRunner, Scratchpad};
pub use mock::{MockSandboxRunner, MockScenario};
pub use wasm::{
    HostState, WasmtimeSandboxRunner, DEFAULT_MAX_FUEL, DEFAULT_MEMORY_CAP_BYTES, DEFAULT_TIMEOUT,
};
