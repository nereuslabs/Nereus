//! Nereus Plugin: Module registry, WebAssembly Component Model host, and hot-swapping coordination.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

pub mod component;
pub mod loader;
pub mod registry;
pub mod wit;

pub use component::{WasmCurriculumComponent, WASM_MAGIC_HEADER};
pub use loader::PluginExtensionLoader;
pub use registry::ModuleRegistry;
pub use wit::{WitChallenge, WitCurriculumManifest, CANONICAL_WIT_SPECIFICATION};
