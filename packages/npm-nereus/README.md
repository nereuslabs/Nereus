# @nereuslabs/nereus

> **The Terminal-First AI Programming Mentor Built in Rust**

[![npm version](https://img.shields.io/npm/v/@nereuslabs/nereus.svg)](https://www.npmjs.com/package/@nereuslabs/nereus)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform Matrix](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-blue)](https://github.com/nereuslabs/nereus)

`@nereuslabs/nereus` is the zero-dependency npm distribution package for **Nereus 2.0**. It bundles a lightweight launcher script and post-install downloader that automatically fetches the appropriate high-performance precompiled Rust native binary for your operating system and CPU architecture.

---

## ⚡ Quick Start

### One-Command Instant Launch (No Install Required)
You can launch Nereus immediately using `npx`:

```bash
npx @nereuslabs/nereus
```

### Run Deterministic Offline Demo Mode
Explore the Socratic coding mentor without needing an LLM API key:

```bash
npx @nereuslabs/nereus --offline --demo
```

### Global Installation
Install Nereus globally on your machine to have the `nereus` command available in any terminal:

```bash
# Using npm
npm install -g @nereuslabs/nereus

# Using pnpm
pnpm add -g @nereuslabs/nereus

# Using yarn
yarn global add @nereuslabs/nereus
```

Then invoke `nereus` from anywhere:

```bash
nereus --help
```

---

## 🖥️ Platform Support Matrix

Nereus automatically detects your operating system and CPU architecture during installation or when running via `npx`:

| Platform | Architecture | Binary Artifact | Status |
| :--- | :--- | :--- | :--- |
| **macOS** | Apple Silicon (`arm64`: M1/M2/M3/M4) | `nereus-v0.2.0-darwin-arm64.tar.gz` | Supported |
| **macOS** | Intel (`x64`) | `nereus-v0.2.0-darwin-x64.tar.gz` | Supported |
| **Linux** | Standard 64-bit (`x64`) | `nereus-v0.2.0-linux-x64.tar.gz` | Supported |
| **Linux** | ARM64 (`aarch64` / `arm64`) | `nereus-v0.2.0-linux-arm64.tar.gz` | Supported |
| **Windows** | 64-bit (`x64`) | `nereus-v0.2.0-win32-x64.tar.gz` | Supported |

---

## ⚙️ How It Works

1. **Lightweight & Dependency-Free**: The `@nereuslabs/nereus` package has **0 runtime dependencies**.
2. **Automated Binary Resolution**: Upon installation (`postinstall`), `install.js` contacts GitHub Releases to download the precompiled binary for the current host environment into `vendor/`.
3. **Transparent Execution**: `bin/nereus.js` forwards your terminal arguments, standard input/output (`stdin`, `stdout`, `stderr`), and process signals (`SIGINT`, `SIGTERM`) directly to the native binary.

### Advanced Configuration via Environment Variables

| Variable | Description |
| :--- | :--- |
| `NEREUS_BIN_PATH` | Explicit absolute path to a custom Nereus executable binary. |
| `NEREUS_SKIP_DOWNLOAD` | Set to `1` or `true` to skip binary download during postinstall. |
| `NEREUS_DOWNLOAD_URL` | Override the default GitHub Releases URL with an internal or mirror URL. |
| `NEREUS_LOCAL_BIN` | Path to a local binary to copy into `vendor/` during postinstall. |

---

## 📖 CLI Usage Overview

```bash
# Launch interactive TUI cockpit
nereus

# Start a targeted session on a specific skill and difficulty
nereus --skill "Rust" --level "intermediate"

# Run in offline demo mode with mock sandbox execution
nereus --offline --demo

# Resume an existing learning session
nereus resume <session-id>

# Display version information
nereus --version
```

---

## 🔗 Related Resources

- **GitHub Repository**: [nereuslabs/nereus](https://github.com/nereuslabs/nereus)
- **Homebrew Tap**: `brew install nereuslabs/tap/nereus`
- **Shell Installer**: `curl -fsSL https://nereuslabs.github.io/install.sh | sh`
- **Documentation**: [https://nereuslabs.github.io](https://nereuslabs.github.io)

---

## 📜 License

MIT © [Nereus Labs](https://github.com/nereuslabs)
