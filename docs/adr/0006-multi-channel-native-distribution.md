# ADR 0006: Multi-Channel Native Distribution (npm, brew, curl)

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-12

---

## Context & Problem Statement

Command-line developer tools must be installable via standard ecosystem channels with zero build-from-source friction. Forcing users to clone a repository and run `cargo install` introduces high barrier to entry for novice or non-Rust developers.

## Decision Drivers

* One-command installation across major operating systems (macOS Apple Silicon/Intel, Linux x86_64/aarch64).
* Support for popular package managers (`npm`/`npx`, `brew`) and direct shell execution (`curl | sh`).

## Considered Options

1. Distribute exclusively via `crates.io` (`cargo install nereus`).
2. Multi-channel distribution: npm wrapper + Homebrew tap + POSIX shell installer.

## Decision Outcome

Chosen option: **Multi-Channel Distribution**:
1. `npm` / `npx`: `@nereuslabs/nereus` package with platform-specific pre-compiled binary extraction (`npx @nereuslabs/nereus`).
2. `Homebrew`: Official tap formula in `Formula/nereus.rb` (`brew install nereuslabs/tap/nereus`).
3. `Shell Installer`: `scripts/install.sh` (`curl -fsSL https://nereus.dev/install.sh | sh`).

### Consequences

* **Positive**: Instant installation without requiring a local Rust toolchain.
* **Positive**: Familiar tooling for web and systems developers alike.
* **Negative**: Requires multi-target cross-compilation release matrix in CI/CD release workflow.
