#!/usr/bin/env node

/**
 * Test suite for @nereuslabs/nereus npm wrapper package.
 */

const assert = require("assert");
const path = require("path");
const fs = require("fs");
const { spawnSync } = require("child_process");

console.log("🌊 Testing @nereuslabs/nereus package integrity...\n");

// 1. Verify package.json structure
console.log("1. Checking package.json metadata...");
const pkgPath = path.join(__dirname, "package.json");
assert(fs.existsSync(pkgPath), "package.json must exist");
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));

assert.strictEqual(pkg.name, "@nereuslabs/nereus");
assert.strictEqual(pkg.version, "0.3.0");
assert.strictEqual(pkg.license, "MIT");
assert.strictEqual(pkg.bin.nereus, "bin/nereus.js");
assert.strictEqual(pkg.scripts.postinstall, "node install.js");
assert.strictEqual(
  pkg.description,
  "The Terminal-First AI Programming Mentor Built in Rust"
);
console.log("   ✓ package.json metadata valid.\n");

// 2. Verify bin/nereus.js exists and is executable
console.log("2. Checking bin/nereus.js...");
const binPath = path.join(__dirname, "bin", "nereus.js");
assert(fs.existsSync(binPath), "bin/nereus.js must exist");
const binContent = fs.readFileSync(binPath, "utf8");
assert(binContent.startsWith("#!/usr/bin/env node"), "bin/nereus.js must have node shebang");
console.log("   ✓ bin/nereus.js valid.\n");

// 3. Verify install.js target mapping and functions
console.log("3. Checking install.js exportable logic...");
const installModule = require("./install.js");
assert(typeof installModule.getTargetKey === "function", "getTargetKey must be a function");
assert(installModule.SUPPORTED_TARGETS["darwin-arm64"], "darwin-arm64 target supported");
assert(installModule.SUPPORTED_TARGETS["darwin-x64"], "darwin-x64 target supported");
assert(installModule.SUPPORTED_TARGETS["linux-x64"], "linux-x64 target supported");
assert(installModule.SUPPORTED_TARGETS["win32-x64"], "win32-x64 target supported");

// Test extractArchive on a synthesized test tarball
const os = require("os");
const tempExtractDir = fs.mkdtempSync(path.join(os.tmpdir(), "nereus-extract-test-"));
const tempTarball = path.join(tempExtractDir, "test.tar.gz");
const testFile = path.join(tempExtractDir, "test.txt");
fs.writeFileSync(testFile, "Nereus packaging test payload");
spawnSync("tar", ["-czf", tempTarball, "-C", tempExtractDir, "test.txt"]);
fs.unlinkSync(testFile);

const unpackDir = path.join(tempExtractDir, "unpacked");
fs.mkdirSync(unpackDir);
installModule.extractArchive(tempTarball, unpackDir);
assert(fs.existsSync(path.join(unpackDir, "test.txt")), "Unpacked file must exist");
assert.strictEqual(
  fs.readFileSync(path.join(unpackDir, "test.txt"), "utf8"),
  "Nereus packaging test payload"
);
fs.rmSync(tempExtractDir, { recursive: true, force: true });
console.log("   ✓ install.js archive extraction verified.\n");

// 4. Test launcher resolution with --version
console.log("4. Testing binary launcher with --version...");
const runResult = spawnSync(process.execPath, [binPath, "--version"], {
  encoding: "utf8",
  env: process.env,
});

console.log("   Launcher stdout:", runResult.stdout.trim());
if (runResult.stderr) {
  console.log("   Launcher stderr:", runResult.stderr.trim());
}
assert.strictEqual(runResult.status, 0, `Launcher must exit with 0, got ${runResult.status}`);
assert(
  runResult.stdout.toLowerCase().includes("nereus") || runResult.stdout.includes("0.3.0"),
  "Launcher output must contain 'nereus' or '0.3.0'"
);
console.log("   ✓ Launcher execution succeeded.\n");

// 5. Test launcher resolution with --help
console.log("5. Testing binary launcher with --help...");
const helpResult = spawnSync(process.execPath, [binPath, "--help"], {
  encoding: "utf8",
  env: process.env,
});
assert.strictEqual(helpResult.status, 0, `Launcher --help must exit with 0, got ${helpResult.status}`);
assert(helpResult.stdout.includes("nereus") || helpResult.stdout.includes("Usage"), "Launcher output must show help");
console.log("   ✓ Launcher --help succeeded.\n");

console.log("✨ All @nereuslabs/nereus tests passed successfully!");
