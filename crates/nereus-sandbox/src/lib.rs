//! Nereus Sandbox: Execution-driven assessment environments.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod ast;
pub mod curriculum;
pub mod diagnostic;
pub mod host;
pub mod mock;
pub mod wasm;

pub use ast::AstInspector;
pub use curriculum::{Challenge, Curriculum};
pub use diagnostic::{CompilerDiagnostic, DiagnosticLevel, DiagnosticParser, DiagnosticSpan};
pub use host::{
    prepare_harness_code, prepare_harness_code_for_topic, HostSandboxRunner, Scratchpad,
};
pub use mock::{MockSandboxRunner, MockScenario};
pub use wasm::{
    HostState, WasmtimeSandboxRunner, DEFAULT_MAX_FUEL, DEFAULT_MEMORY_CAP_BYTES, DEFAULT_TIMEOUT,
};
