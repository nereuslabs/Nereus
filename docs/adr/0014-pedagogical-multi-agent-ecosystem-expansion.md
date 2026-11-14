# ADR 0014: Pedagogical Multi-Agent Ecosystem Expansion: Librarian and Adversary

## Status
Accepted

## Context
Nereus 2.0 was founded upon four specialized pedagogical agents: `DiagnosticAgent`, `CoachAgent`, `TutorAgent`, and `ExaminerAgent`. As learners progress to intermediate and advanced systems programming topics (lifetimes, unsafe memory manipulation, concurrency primitives), two critical learning vectors emerged:
1. **Instant Offline Documentation Reference**: Learners frequently encounter obscure compiler errors or standard library abstractions (`std::sync::Arc`, `std::pin::Pin`, Nomicon subtyping) and need context-aware citations without opening an external browser or relying on cloud network connections.
2. **Defensive Programming & Edge-Case Resilience**: Passing a nominal test suite often masks fragile code (e.g. integer arithmetic overflows, empty slice panics, race condition preemption). Learners need an adversarial sparring partner that injects pathological stress cases.

## Decision
We expanded the multi-agent ecosystem in `nereus-agents` with two specialized autonomous agents:
1. **`LibrarianAgent`**:
   - Maintains an in-memory offline corpus covering the Rust Standard Library (`std::sync`, `std::cell`, `std::pin`, `std::marker`, `std::ops`), The Rust Nomicon, and the Rust Reference.
   - Provides exact symbol lookups (`lookup_symbol`), BM25-ranked token search (`search`), and compiler diagnostic citations (`cite_for_error`).
   - Synthesizes Socratic inquiry notes highlighting ownership and memory safety invariants.
2. **`AdversaryAgent`**:
   - Operates as an educational boundary-testing and property-based fuzzing agent.
   - Analyzes learner code and topics to generate pathological stress harnesses across five vectors: `BoundaryCondition`, `ConcurrencyStress`, `MemorySafety`, `ResourceExhaustion`, and `ArithmeticOverflow`.
   - Challenges learners with Socratic warnings and defensive programming guidelines before advancing to subsequent curriculum milestones.

## Consequences
### Positive
- **100% Offline Technical Inquiry**: Documentation citations and error code references require zero network connectivity and zero token expenditure.
- **Deepened Rigor**: The Adversary Agent trains defensive programming habits and highlights edge cases (e.g. empty collections, integer extremes, lock contention).
- **Extensible Architecture**: Clean agent boundaries adhere to the core `Agent` trait contracts.

### Negative
- Curated documentation corpus requires periodic manual auditing as Rust editions and standard library APIs evolve.
