# ADR 0001: Record Architecture Decisions

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-09

---

## Context & Problem Statement

As Nereus evolved from a Python v1 prototype to a 9-crate idiomatic Rust workspace with multiple subagents and contributors, architectural decisions (such as choosing memory-safe sandboxing, algebraic state machines, and terminal UIs) needed a formalized, discoverable, and immutable record.

## Decision Drivers

* Maintain historical continuity across agent sessions and human contributors.
* Prevent regressions into architectural anti-patterns (e.g., dynamic typing, unconstrained subprocess execution).
* Provide dual synchronization between local repository docs and the cloud Notion Cockpit.

## Considered Options

1. Informal commit messages and PR descriptions.
2. Architecture Decision Records (MADR format in `docs/adr/`).
3. Single monolithic architecture document without versioned decision records.

## Decision Outcome

Chosen option: **MADR format in `docs/adr/`** + continuous dual synchronization with Notion Cockpit (`41300ced-bff4-8178-a81f-c6d215179c84`).

### Consequences

* **Positive**: Every significant choice is documented with context, alternatives, and explicit tradeoffs.
* **Positive**: New AI agents and contributors can immediately parse the rationale behind technical constraints.
* **Negative**: Requires discipline to document decisions prior to or alongside merging architectural pull requests.
