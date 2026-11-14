# ADR 0002: Compile-Time Algebraic State Machine over LangGraph

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-09

---

## Context & Problem Statement

Nereus v1 utilized Python and LangGraph (`StateGraph`) for pedagogical agent orchestration. In practice, LangGraph suffered from:
1. **Opaque Dynamic Typing & State Drift**: Graph state was represented as mutable `TypedDict` serialized via checkpointers. Schema evolution regularly corrupted stored sessions.
2. **Interrupt Fragility**: Human-in-the-loop interactions relied on Python exception unwinding via `interrupt()`, leading to brittle JSON unpacking and unpredictable state restores.
3. **Bloated Runtime**: Python runtime dependencies (`torch`, `chromadb`, `pydantic`) demanded >350MB RAM and 3-second startup delays, incompatible with a nimble terminal utility.

## Decision Drivers

* Instant startup (<5ms) and tiny memory footprint (<20MB).
* Zero unhandled runtime states or silent field drops.
* Deterministic serializability for session persistence.

## Considered Options

1. Refactor Python LangGraph with strict Pydantic v2 validation.
2. Rewrite in Go using channel-based goroutines.
3. Rewrite in Rust using an algebraic finite state automaton (`enum LearningState`).

## Decision Outcome

Chosen option: **Rust Algebraic Finite State Machine (`enum LearningState`)** in `nereus-core` and `nereus-engine`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LearningState {
    Diagnostic { questions: Vec<DiagnosticQuestion>, current_index: usize, answers: Vec<String> },
    Coaching { weakness_report: WeaknessReport },
    Tutoring { topic: RoadmapTopic, material: String },
    Examining { topic: RoadmapTopic, attempts: usize, last_feedback: Option<String> },
    Completed,
}
```

### Consequences

* **Positive**: Invalid states and transitions cannot compile.
* **Positive**: Native binary startup drops from ~3,500ms to <4ms. Memory footprint drops from 350MB to ~12MB.
* **Positive**: Serialization and deserialization are verified at compile-time via `serde`.
* **Negative**: State transitions must be explicitly declared and handled across all pattern matches.
