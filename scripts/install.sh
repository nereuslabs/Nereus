#!/bin/sh
# Nereus 2.0 POSIX Single-Line Shell Installer
# Usage:
#   curl -fsSL https://nereuslabs.github.io/install.sh | sh
#
# Supported Platforms:
#   - macOS (Apple Silicon arm64 & Intel x64)
#   - Linux (x64 & arm64)

set -e

REPO="nereuslabs/nereus"
FALLBACK_VERSION="v0.2.0"
DEST_DIR="${NEREUS_INSTALL_DIR:-$HOME/.local/bin}"

# ANSI Colors (disabled if stdout is not a TTY)
if [ -t 1 ]; then
  BOLD="\033[1m"
  GREEN="\033[32m"
  YELLOW="\033[33m"
  CYAN="\033[36m"
  RED="\033[31m"
  RESET="\033[0m"
else
  BOLD=""
  GREEN=""
  YELLOW=""
  CYAN=""
  RED=""
  RESET=""
fi

echo "${CYAN}${BOLD}🌊 Nereus 2.0 Terminal Coding Mentor Installer${RESET}"
echo "=================================================="

# 1. Detect Operating System
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
  darwin)
    OS="darwin"
    ;;
  linux)
    OS="linux"
    ;;
  *)
    echo "${RED}Error: Unsupported operating system: ${OS}.${RESET}"
    echo "Nereus currently supports macOS and Linux via this shell installer."
    echo "For Windows, install via npm: npm install -g @nereuslabs/nereus"
    exit 1
    ;;
esac

# 2. Detect CPU Architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)
    ARCH="x64"
    ;;
  arm64|aarch64)
    ARCH="arm64"
    ;;
  *)
    echo "${RED}Error: Unsupported CPU architecture: ${ARCH}.${RESET}"
    echo "Supported architectures: x64, arm64."
    exit 1
    ;;
esac

echo "Detected Platform: ${BOLD}${OS}-${ARCH}${RESET}"

# 3. Setup temporary workspace and trap cleanup
TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'nereus-install')"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

# 4. Handle Local Test / Override Mode
if [ -n "$NEREUS_LOCAL_BIN" ] && [ -f "$NEREUS_LOCAL_BIN" ]; then
  echo "Using local binary override: ${NEREUS_LOCAL_BIN}"
  mkdir -p "$DEST_DIR"
  cp "$NEREUS_LOCAL_BIN" "$DEST_DIR/nereus"
  chmod +x "$DEST_DIR/nereus"
  echo "${GREEN}✓ Installed local binary to ${DEST_DIR}/nereus${RESET}"
else
  # 5. Resolve Release Version
  if [ -n "$NEREUS_VERSION" ]; then
    TAG="$NEREUS_VERSION"
    # Ensure leading 'v' in tag
    case "$TAG" in
      v*) ;;
      *) TAG="v$TAG" ;;
    esac
  else
    # Fetch latest release tag from GitHub API
    echo "Resolving latest release from GitHub..."
    if command -v curl >/dev/null 2>&1; then
      TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)"
    elif command -v wget >/dev/null 2>&1; then
      TAG="$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)"
    fi

    if [ -z "$TAG" ]; then
      TAG="$FALLBACK_VERSION"
      echo "${YELLOW}Notice: Could not resolve latest tag from GitHub API; falling back to ${TAG}.${RESET}"
    fi
  fi

  TARBALL="nereus-${TAG}-${OS}-${ARCH}.tar.gz"
  URL="${NEREUS_DOWNLOAD_URL:-https://github.com/${REPO}/releases/download/${TAG}/${TARBALL}}"

  echo "Downloading ${BOLD}${TAG}${RESET} (${OS}-${ARCH})..."
  echo "URL: ${URL}"

  DOWNLOAD_SUCCESS=0
  if command -v curl >/dev/null 2>&1; then
    if curl -fL "$URL" -o "$TMP_DIR/$TARBALL" 2>/dev/null; then
      DOWNLOAD_SUCCESS=1
    fi
  elif command -v wget >/dev/null 2>&1; then
    if wget -qO "$TMP_DIR/$TARBALL" "$URL" 2>/dev/null; then
      DOWNLOAD_SUCCESS=1
    fi
  else
    echo "${RED}Error: Neither curl nor wget was found on your system.${RESET}"
    echo "Please install curl or wget to continue."
    exit 1
  fi

  if [ "$DOWNLOAD_SUCCESS" -ne 1 ]; then
    echo "${YELLOW}Warning: Prebuilt release archive could not be downloaded from GitHub.${RESET}"
    echo "URL attempted: ${URL}"

    # Check if a local workspace build exists to recover
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
    if [ -f "$WORKSPACE_DIR/target/release/nereus" ]; then
      echo "Found local release build at: $WORKSPACE_DIR/target/release/nereus"
      cp "$WORKSPACE_DIR/target/release/nereus" "$TMP_DIR/nereus"
    elif [ -f "$WORKSPACE_DIR/target/debug/nereus" ]; then
      echo "Found local debug build at: $WORKSPACE_DIR/target/debug/nereus"
      cp "$WORKSPACE_DIR/target/debug/nereus" "$TMP_DIR/nereus"
    else
      echo "${RED}Error: Failed to obtain Nereus binary.${RESET}"
      echo "You can build Nereus from source with:"
      echo "  cargo install nereus-cli"
      exit 1
    fi
  else
    echo "Extracting release archive..."
    tar -xzf "$TMP_DIR/$TARBALL" -C "$TMP_DIR"
  fi

  if [ ! -f "$TMP_DIR/nereus" ]; then
    echo "${RED}Error: nereus binary was not found after archive extraction.${RESET}"
    exit 1
  fi

  # 6. Copy binary to destination
  echo "Installing binary to ${DEST_DIR}..."
  mkdir -p "$DEST_DIR"
  cp "$TMP_DIR/nereus" "$DEST_DIR/nereus"
  chmod +x "$DEST_DIR/nereus"
fi

# 7. Post-Installation Verification & Path Guidance
echo ""
echo "${GREEN}${BOLD}✨ Nereus was successfully installed to:${RESET} ${DEST_DIR}/nereus"
echo ""

# Check if destination directory is in PATH
case ":$PATH:" in
  *":$DEST_DIR:"*)
    echo "${GREEN}✓ ${DEST_DIR} is already in your PATH.${RESET}"
    ;;
  *)
    echo "${YELLOW}⚠️  ${DEST_DIR} is NOT in your current PATH.${RESET}"
    echo "To run 'nereus' from any terminal, add the following line to your shell configuration:"
    echo ""
    if [ -n "$ZSH_VERSION" ] || [ "$(basename "$SHELL" 2>/dev/null)" = "zsh" ]; then
      echo "  ${CYAN}echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc${RESET}"
      echo "  ${CYAN}source ~/.zshrc${RESET}"
    elif [ -n "$BASH_VERSION" ] || [ "$(basename "$SHELL" 2>/dev/null)" = "bash" ]; then
      echo "  ${CYAN}echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc${RESET}"
      echo "  ${CYAN}source ~/.bashrc${RESET}"
    else
      echo "  ${CYAN}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
    fi
    echo ""
    ;;
esac

echo "Quick Start Commands:"
echo "  ${BOLD}nereus --help${RESET}          # Display command-line options"
echo "  ${BOLD}nereus --offline --demo${RESET} # Launch offline Socratic demo"
echo "  ${BOLD}nereus${RESET}                 # Launch interactive TUI cockpit"
echo ""
