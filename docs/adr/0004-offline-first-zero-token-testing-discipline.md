# ADR 0004: Offline-First Zero-Token Testing Discipline

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-10

---

## Context & Problem Statement

AI-driven developer tools often suffer from fragile, slow, and expensive test suites that invoke live commercial LLM APIs (e.g., Anthropic, OpenAI, OpenRouter) during CI and local testing. This introduces:
1. Flaky tests caused by API latency, rate limits, or transient outages.
2. Uncontrollable token costs on every commit or PR.
3. Secret management risks (leaking API keys into test environments).

## Decision Drivers

* 100% deterministic test execution in CI and offline developer environments.
* Zero financial cost to run unit and integration tests.
* Full test coverage for agent logic, prompt rendering, JSON retry loops, and state transitions.

## Considered Options

1. Live API integration tests guarded by secrets.
2. HTTP-level VCR record/replay mocks.
3. In-memory trait polymorphism via `StubLLMClient` and `MockSandboxRunner`.

## Decision Outcome

Chosen option: **Deterministic Mock Trait Polymorphism**:
All core subsystems (`nereus-engine`, `nereus-agents`, `nereus-cli`, `nereus-tui`) depend solely on the `LlmClient` and `SandboxRunner` abstractions defined in `nereus-core`.

* `StubLLMClient`: In-memory deterministic client returning pre-configured or dynamic structured responses without network access.
* `MockSandboxRunner`: Configurable runner with simulated compiler errors, timeouts, or success states.

### Consequences

* **Positive**: 100% of the 100+ workspace tests execute in <3 seconds without internet connectivity or token expenditure.
* **Positive**: CI pipelines run hermetically in GitHub Actions with zero API credentials configured.
* **Negative**: Live inference validation must be performed in dedicated manual verification or scheduled staging runs.
