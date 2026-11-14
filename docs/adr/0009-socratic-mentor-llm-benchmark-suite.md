# ADR 0009: Hermetic LLM Socratic Mentor Benchmark Suite and Solution Leakage Detection

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-14

---

## Context & Problem Statement

LLMs frequently suffer from pedagogical regression: instead of guiding the learner through Socratic questioning and conceptual hints, they often leak the exact code solution. In an educational programming mentor, solution leakage ruins the learning loop by removing active problem-solving.

Furthermore, evaluating LLM mentor performance typically requires expensive, non-deterministic online API calls, making continuous integration (CI) slow, costly, and fragile.

## Decision Drivers

* Rigorous, automated detection of direct code leakage (code blocks, drop-in syntax fixes).
* High-precision evaluation of diagnostic error identification (e.g., E0382 borrow after move, E0597 dropped value borrowed).
* Hermetic offline benchmark verification runnable in CI within milliseconds without token consumption.
* Unified CLI command (`nereus bench`) supporting both JSON and Markdown reporting.

## Considered Options

1. **LLM-as-a-Judge**: Prompting a secondary LLM (e.g., GPT-4) to grade the mentor's response. Inherently non-deterministic, expensive, and unsuited for offline CI.
2. **Deterministic Heuristic Rubric with Leakage Scanner**: A standardized problem bank of 10 canonical Rust compiler mistakes evaluated against regex-based leakage detectors, diagnostic keyword matching, and Socratic questioning metrics.

## Decision Outcome

Chosen option: **Deterministic Heuristic Rubric with Leakage Scanner** in `nereus-agents::benchmark` and `nereus-cli::bench`.

The benchmark problem bank encompasses 10 canonical Rust compiler errors:
- E0382: Use of moved value
- E0502: Cannot borrow mutable because also borrowed as immutable
- E0597: Borrowed value does not live long enough
- E0277: Trait bound not satisfied
- E0308: Mismatched types
- E0061: This function takes N arguments but M arguments were supplied
- E0499: Cannot borrow as mutable more than once at a time
- E0515: Cannot return value referencing local variable
- E0106: Missing lifetime specifier
- E0716: Temporary value dropped while borrowed

Each test case verifies:
1. **Diagnostic Precision**: Does the mentor pinpoint the specific compiler error code and concept?
2. **Guidance Score**: Does the mentor ask probing questions or provide conceptual metaphors?
3. **Zero Solution Leakage**: Does the mentor avoid emitting markdown code blocks containing the exact fix?

### Consequences

* **Positive**: Fast offline execution (runs in ~15ms via `StubLLMClient`).
* **Positive**: Prevents pedagogical regression during prompt engineering or model switching.
* **Positive**: Structured output via `--format json` or `--format markdown` suitable for automated dashboard tracking.
* **Negative**: Heuristic rules must be periodically maintained when adding new programming languages or novel error paradigms.
