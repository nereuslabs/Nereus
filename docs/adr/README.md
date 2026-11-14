# Architecture Decision Records (ADRs)

This directory documents the foundational architectural and technical decisions made in **Nereus 2.0**.

We adhere to the [Markdown Any Decision Record (MADR)](https://adr.github.io/madr/) format.

---

## Index of Decisions

| ADR | Title | Status | Date |
| :---: | :--- | :---: | :---: |
| [`0001`](0001-record-architecture-decisions.md) | Record Architecture Decisions | **Accepted** | 2026-11-09 |
| [`0002`](0002-compile-time-algebraic-state-machine-over-langgraph.md) | Compile-Time Algebraic State Machine over LangGraph | **Accepted** | 2026-11-09 |
| [`0003`](0003-wasmtime-and-host-compiler-dual-sandbox.md) | Wasmtime & Host Compiler Dual Execution Sandbox | **Accepted** | 2026-11-10 |
| [`0004`](0004-offline-first-zero-token-testing-discipline.md) | Offline-First Zero-Token Testing Discipline | **Accepted** | 2026-11-10 |
| [`0005`](0005-ratatui-split-pane-terminal-cockpit.md) | Ratatui Split-Pane Terminal Cockpit over Web UI | **Accepted** | 2026-11-11 |
| [`0006`](0006-multi-channel-native-distribution.md) | Multi-Channel Native Distribution (npm, brew, curl) | **Accepted** | 2026-11-12 |

---

## Dual Synchronization with Cloud (Notion Cockpit)

ADR records must remain synchronized between Git (`docs/adr/`) and the project's **Notion Status Cockpit**:
- **Notion Page ID**: `41300ced-bff4-8178-a81f-c6d215179c84`
- **Notion Page URL**: [https://app.notion.com/p/41300cedbff48178a81fc6d215179c84?pvs=204](https://app.notion.com/p/41300cedbff48178a81fc6d215179c84?pvs=204)
