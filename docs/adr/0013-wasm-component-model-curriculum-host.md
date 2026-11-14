# ADR 0013: WebAssembly Component Model Curriculum Host and Dynamic Extension Loader

## Status
Accepted

## Context
Nereus 2.0 originally shipped with a curated in-tree curriculum bundled directly into `nereus-sandbox`. However, external educational institutions, university courses, and technical corporate bootcamps require the ability to author, distribute, and grade custom domain tracks (e.g., lock-free concurrency, async runtime internals, embedded systems) without recompiling or modifying the Nereus core binary.

We needed an extensible plugin architecture that guarantees:
1. Sandboxed memory safety and deterministic evaluation.
2. Standardized type contracts across programming languages.
3. Zero native compilation toolchain requirements on the learner's machine.

## Decision
We adopted the **WebAssembly Component Model** with declarative **WebAssembly Interface Types (WIT)**:
1. Defined `package nereus:curriculum@0.3.0` specifying the `topic-provider` interface:
   - `record challenge`: Contains exercise identity, title, difficulty, starter scaffold, test harness, and concept tags.
   - `get-manifest`: Returns the sequence of curriculum topics.
   - `evaluate-solution`: Evaluates candidate code against challenge invariants.
2. Implemented `WasmCurriculumComponent` in `nereus-plugin` to validate `\0asm` binary headers, parse paired metadata manifests, and enforce execution boundaries.
3. Implemented `PluginExtensionLoader` scanning `~/.nereus/plugins/` to discover, validate, and inject external `.wasm` curriculum packages into `ModuleRegistry`.

## Consequences
### Positive
- **Language Agnostic**: Educators can compile curricula from Rust, C/C++, Zig, or Go into standard `.wasm` components.
- **Air-Gapped Safety**: Untrusted course modules run within Wasm boundaries without host filesystem or network access.
- **Plug-and-Play Distribution**: Dropping a `.wasm` file into `~/.nereus/plugins/` immediately expands the learner's curriculum options.

### Negative
- Component Model tooling (`wit-bindgen`, wasm component tooling) is evolving and requires explicit adherence to canonical WIT schemas.
