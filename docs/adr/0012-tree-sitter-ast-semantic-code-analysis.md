# ADR 0012: Tree-Sitter AST Static Analysis & Code Anti-Pattern Detection

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-14

---

## Context & Problem Statement

In Nereus 2.0, code validation relies primarily on `rustc` compiler diagnostics and runtime assertion execution in the sandbox. While effective at enforcing syntactical correctness and type safety, the compiler cannot detect educational anti-patterns (such as redundant `.clone()`, unidiomatic loops instead of iterators, unhandled `unwrap()`, or premature `unsafe` usage). A student can produce code that passes compiler tests while remaining architecturally flawed or non-idiomatic.

## Decision Drivers

* Instant syntactic and semantic analysis before invoking `rustc` (<10ms).
* Exact source range pinpointing for code smell highlights.
* Declarative pedagogical query rules that can evolve alongside new curriculum exercises.

## Considered Options

1. **Clippy as a Subprocess (`cargo clippy --message-format=json`)**: Provides exhaustive warnings, but is slow (~1-2 seconds per invocation) and requires disk I/O and full dependency tree resolution.
2. **Regex / Substring Pattern Matching**: Lightweight and fast, but brittle, prone to false positives in comments/strings, and unaware of lexical scope.
3. **In-Memory Concrete Syntax Tree (CST) parsing via `tree-sitter-rust`**: Parses source buffers in sub-millisecond time directly in memory and allows declarative S-expression pattern queries.

## Decision Outcome

Chosen option: **In-Memory Concrete Syntax Tree (CST) parsing via `tree-sitter-rust`** in `nereus-sandbox::ast`.

```rust
pub struct AstInspector {
    parser: tree_sitter::Parser,
    query_set: Vec<PedagogicalRule>,
}

pub struct PedagogicalSmell {
    pub rule_id: String,
    pub line: usize,
    pub column: usize,
    pub socratic_hint: String,
}
```

The Examiner Agent queries the `AstInspector` prior to sandboxed compilation. If severe anti-patterns are detected, the Examiner weaves targeted Socratic questions into the feedback loop.

### Consequences

* **Positive**: Sub-millisecond analysis (~2ms) entirely in memory.
* **Positive**: Detects unidiomatic code that the compiler would otherwise accept without complaint.
* **Positive**: Enables visual squiggly underlines and gutter markers in the Ratatui code editor buffer.
* **Negative**: Requires maintaining `tree-sitter-rust` C grammar bindings as a Cargo dependency.
