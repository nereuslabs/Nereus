# Nereus 2.0 Packaging & Multi-Channel Distribution Architecture

> **Objective**: Enable zero-friction, one-command installation of the Nereus terminal coding cockpit across macOS, Linux, and Windows without requiring end-users to have the Rust toolchain installed.

---

## 1. Distribution Channels Matrix

| Channel | Install Command | Target Audience | Mechanism |
| :--- | :--- | :--- | :--- |
| **npm / npx** | `npm install -g @nereuslabs/nereus`<br>`npx @nereuslabs/nereus` | JavaScript / TypeScript / Web devs | Lightweight npm package with platform-specific native binary auto-fetcher |
| **Homebrew** | `brew install nereuslabs/tap/nereus` | macOS & Linux Homebrew users | Homebrew Tap formula downloading precompiled binary tarball from GitHub Releases |
| **Shell Installer** | `curl -fsSL https://nereuslabs.github.io/install.sh \| sh` | General Unix / Linux / macOS | POSIX shell script detecting OS/arch and installing binary to `~/.local/bin` |
| **Cargo / Crates.io** | `cargo install nereus-cli`<br>`cargo binstall nereus-cli` | Rust community | Direct crate build or binary install via `cargo-binstall` |
| **GitHub Releases** | Manual download from Releases page | Systems engineers, air-gapped CI | Static archives (`.tar.gz`, `.zip`) with SHA-256 checksums |

---

## 2. Channel A: npm / npx Binary Wrapper (`@nereuslabs/nereus`)

The npm distribution pattern mirrors industry-standard tools like Turbo, Biome, and Tailwind CLI:

### 2.1. Package Architecture
```
packages/npm-nereus/
├── package.json          # Root package publishing binary wrapper
├── README.md
├── bin/
│   └── nereus.js         # Platform detection launcher script
└── install.js            # Post-install hook downloading appropriate binary
```

### 2.2. Platform Resolution Matrix
The launcher inspects `process.platform` and `process.arch` to download or launch the matching binary:
* `darwin-arm64`: Apple Silicon (M1/M2/M3/M4)
* `darwin-x64`: Intel Mac
* `linux-x64`: Standard 64-bit Linux (glibc 2.31+)
* `linux-arm64`: ARM64 Linux (AWS Graviton, Raspberry Pi)
* `win32-x64`: 64-bit Windows (`nereus.exe`)

### 2.3. Launcher Script (`bin/nereus.js`)
```javascript
#!/usr/bin/env node
const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const binName = process.platform === "win32" ? "nereus.exe" : "nereus";
const binPath = path.join(__dirname, "..", "vendor", binName);

if (!fs.existsSync(binPath)) {
  console.error("Nereus native binary not found. Please reinstall or run install.js.");
  process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), { stdio: "inherit" });
child.on("exit", (code) => process.exit(code ?? 0));
```

---

## 3. Channel B: Homebrew Tap (`nereuslabs/homebrew-tap`)

For macOS users, a dedicated Homebrew formula provides native package management and auto-updating.

### 3.1. Tap Repository Setup
Repository: `nereuslabs/homebrew-tap`  
Formula path: `Formula/nereus.rb`

### 3.2. Formula Specification (`Formula/nereus.rb`)
```ruby
class Nereus < Formula
  desc "Terminal-First AI Programming Mentor Built in Rust"
  homepage "https://nereuslabs.github.io"
  version "0.3.0"
  license "MIT"

  if OS.mac? && Hardware::CPU.arm?
    url "https://github.com/nereuslabs/nereus/releases/download/v0.3.0/nereus-v0.3.0-darwin-arm64.tar.gz"
    sha256 "REPLACE_WITH_ACTUAL_SHA256_ARM64"
  elsif OS.mac? && Hardware::CPU.intel?
    url "https://github.com/nereuslabs/nereus/releases/download/v0.3.0/nereus-v0.3.0-darwin-x64.tar.gz"
    sha256 "REPLACE_WITH_ACTUAL_SHA256_X64"
  elsif OS.linux? && Hardware::CPU.intel?
    url "https://github.com/nereuslabs/nereus/releases/download/v0.3.0/nereus-v0.3.0-linux-x64.tar.gz"
    sha256 "REPLACE_WITH_ACTUAL_SHA256_LINUX_X64"
  end

  def install
    bin.install "nereus"
  end

  test do
    assert_match "Nereus", shell_output("#{bin}/nereus --version")
  end
end
```

---

## 4. Channel C: Single-Line POSIX Shell Installer

Hosted on GitHub Pages (`https://nereuslabs.github.io/install.sh`):

```bash
#!/bin/sh
set -e

REPO="nereuslabs/nereus"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64) ARCH="x64" ;;
  arm64|aarch64) ARCH="arm64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
TARBALL="nereus-${TAG}-${OS}-${ARCH}.tar.gz"
URL="https://github.com/$REPO/releases/download/${TAG}/${TARBALL}"

echo "🌊 Downloading Nereus ${TAG} for ${OS}-${ARCH}..."
curl -fsSL "$URL" -o "/tmp/$TARBALL"

DEST_DIR="$HOME/.local/bin"
mkdir -p "$DEST_DIR"
tar -xzf "/tmp/$TARBALL" -C "$DEST_DIR"
chmod +x "$DEST_DIR/nereus"

echo "✨ Nereus installed successfully to $DEST_DIR/nereus!"
echo "Make sure $DEST_DIR is in your PATH."
```

---

## 5. Automated GitHub Actions Release Packaging

Whenever a release tag `vX.Y.Z` is pushed to `main`:
1. **Cross-Compilation Matrix**:
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-unknown-linux-gnu`
   - `x86_64-pc-windows-msvc`
2. **Artifact Packaging**:
   - Strip debug symbols: `strip target/release/nereus`
   - Create compressed archives: `tar -czvf nereus-vX.Y.Z-<target>.tar.gz nereus`
   - Compute SHA-256 checksums: `shasum -a 256 *.tar.gz > checksums.txt`
3. **Automated Publishing**:
   - Attach binaries and checksums to GitHub Release.
   - Update formula in `nereuslabs/homebrew-tap` with new release URLs and SHA-256 hashes.
   - Publish wrapper to npm registry (`@nereuslabs/nereus`).
