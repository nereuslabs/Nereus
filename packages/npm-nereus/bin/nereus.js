#!/usr/bin/env node

/**
 * Nereus Native Binary Launcher
 *
 * Detects current OS platform and architecture, locates the prebuilt or
 * locally compiled Nereus native binary, and executes it via child_process.spawn.
 */

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

function resolveBinary() {
  const isWindows = process.platform === "win32";
  const binName = isWindows ? "nereus.exe" : "nereus";

  // 1. Explicit override via environment variable
  if (process.env.NEREUS_BIN_PATH && fs.existsSync(process.env.NEREUS_BIN_PATH)) {
    return process.env.NEREUS_BIN_PATH;
  }

  // 2. Installed prebuilt binary in package vendor/ directory
  const vendorBin = path.join(__dirname, "..", "vendor", binName);
  if (fs.existsSync(vendorBin)) {
    return vendorBin;
  }

  // 3. Monorepo/workspace development builds (target/release or target/debug)
  const candidateRoots = [
    path.resolve(__dirname, "..", ".."),
    path.resolve(__dirname, "..", "..", ".."),
  ];
  for (const root of candidateRoots) {
    const rel = path.join(root, "target", "release", binName);
    if (fs.existsSync(rel)) return rel;
    const dbg = path.join(root, "target", "debug", binName);
    if (fs.existsSync(dbg)) return dbg;
  }

  return null;
}

function main() {
  const binPath = resolveBinary();

  if (!binPath) {
    const isWindows = process.platform === "win32";
    const binName = isWindows ? "nereus.exe" : "nereus";
    const expectedPath = path.join(__dirname, "..", "vendor", binName);

    console.error("\x1b[31m[nereus] Error: Native binary not found.\x1b[0m");
    console.error(`Expected native binary at: ${expectedPath}`);
    console.error("Platform: " + process.platform + ", Architecture: " + process.arch);
    console.error("\nTo resolve this issue:");
    console.error("  1. Run the post-install script: node install.js");
    console.error("  2. Or reinstall: npm install -g @nereuslabs/nereus --force");
    console.error("  3. Or build locally: cargo build --release -p nereus-cli");
    process.exit(1);
  }

  // Spawn the native binary forwarding all arguments and standard streams
  const child = spawn(binPath, process.argv.slice(2), {
    stdio: "inherit",
    env: process.env,
  });

  child.on("error", (err) => {
    console.error(`\x1b[31m[nereus] Failed to start native process: ${err.message}\x1b[0m`);
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code ?? 0);
    }
  });

  // Handle process termination signals cleanly
  const forwardSignal = (sig) => {
    if (!child.killed) {
      child.kill(sig);
    }
  };

  process.on("SIGINT", () => forwardSignal("SIGINT"));
  process.on("SIGTERM", () => forwardSignal("SIGTERM"));
}

if (require.main === module) {
  main();
}

module.exports = { resolveBinary, main };
