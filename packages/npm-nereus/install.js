#!/usr/bin/env node

/**
 * Nereus Post-Install Binary Downloader
 *
 * Downloads the precompiled native Nereus binary from GitHub Releases
 * matching the host operating system and CPU architecture into the vendor/ directory.
 *
 * Supported targets:
 *   - darwin-arm64 (Apple Silicon)
 *   - darwin-x64   (Intel Mac)
 *   - linux-x64    (64-bit Linux)
 *   - linux-arm64  (64-bit ARM Linux)
 *   - win32-x64    (64-bit Windows)
 */

const fs = require("fs");
const path = require("path");
const https = require("https");
const http = require("http");
const { spawnSync } = require("child_process");
const pkg = require("./package.json");

const REPO = "nereuslabs/nereus";
const VERSION = pkg.version;
const TAG = `v${VERSION}`;

const SUPPORTED_TARGETS = {
  "darwin-arm64": { os: "darwin", arch: "arm64", ext: "tar.gz", bin: "nereus" },
  "darwin-x64": { os: "darwin", arch: "x64", ext: "tar.gz", bin: "nereus" },
  "linux-x64": { os: "linux", arch: "x64", ext: "tar.gz", bin: "nereus" },
  "linux-arm64": { os: "linux", arch: "arm64", ext: "tar.gz", bin: "nereus" },
  "win32-x64": { os: "win32", arch: "x64", ext: "tar.gz", bin: "nereus.exe" },
};

function getTargetKey() {
  const platform = process.platform;
  const arch = process.arch;
  const key = `${platform}-${arch}`;

  if (SUPPORTED_TARGETS[key]) {
    return key;
  }
  return null;
}

function ensureDirectory(dirPath) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }
}

/**
 * Downloads a file following HTTP 3xx redirects.
 */
function downloadFile(url, destPath, maxRedirects = 5) {
  return new Promise((resolve, reject) => {
    if (maxRedirects <= 0) {
      return reject(new Error("Too many HTTP redirects encountered."));
    }

    const client = url.startsWith("https:") ? https : http;
    const req = client.get(
      url,
      {
        headers: {
          "User-Agent": `@nereuslabs/nereus-installer/${VERSION} (Node.js ${process.version})`,
          Accept: "application/octet-stream, application/gzip, application/x-tar, */*",
        },
      },
      (res) => {
        // Handle HTTP redirects (301, 302, 307, 308)
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          let redirectUrl = res.headers.location;
          if (!redirectUrl.startsWith("http://") && !redirectUrl.startsWith("https://")) {
            const parsed = new URL(url);
            redirectUrl = new URL(redirectUrl, parsed.origin).href;
          }
          res.resume(); // Drain stream
          return resolve(downloadFile(redirectUrl, destPath, maxRedirects - 1));
        }

        if (res.statusCode !== 200) {
          res.resume();
          return reject(
            new Error(`HTTP ${res.statusCode} ${res.statusMessage || ""}`)
          );
        }

        const fileStream = fs.createWriteStream(destPath);
        res.pipe(fileStream);

        fileStream.on("finish", () => {
          fileStream.close((err) => {
            if (err) reject(err);
            else resolve();
          });
        });

        fileStream.on("error", (err) => {
          fs.unlink(destPath, () => {});
          reject(err);
        });
      }
    );

    req.on("error", (err) => {
      fs.unlink(destPath, () => {});
      reject(err);
    });

    req.setTimeout(60000, () => {
      req.destroy();
      fs.unlink(destPath, () => {});
      reject(new Error("Download request timed out after 60 seconds."));
    });
  });
}

/**
 * Extracts a tar.gz archive to the destination directory.
 */
function extractArchive(archivePath, destDir) {
  const result = spawnSync("tar", ["-xzf", archivePath, "-C", destDir], {
    stdio: "inherit",
  });

  if (result.error) {
    throw new Error(`Failed to execute tar: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`tar extraction failed with exit code ${result.status}`);
  }
}

/**
 * Copies a local development binary if available.
 */
function tryCopyLocalBinary(vendorDir, binName) {
  const candidateRoots = [
    path.resolve(__dirname, "..", ".."),
    path.resolve(__dirname, "..", "..", ".."),
  ];

  const candidates = [
    process.env.NEREUS_LOCAL_BIN,
  ];

  for (const root of candidateRoots) {
    candidates.push(path.join(root, "target", "release", binName));
    candidates.push(path.join(root, "target", "debug", binName));
  }

  for (const candidate of candidates.filter(Boolean)) {
    if (fs.existsSync(candidate)) {
      const dest = path.join(vendorDir, binName);
      fs.copyFileSync(candidate, dest);
      if (process.platform !== "win32") {
        fs.chmodSync(dest, 0o755);
      }
      console.log(`[nereus] Found local development binary at: ${candidate}`);
      console.log(`[nereus] Copied to: ${dest}`);
      return true;
    }
  }
  return false;
}

async function install() {
  const vendorDir = path.join(__dirname, "vendor");
  ensureDirectory(vendorDir);

  const targetKey = getTargetKey();
  if (!targetKey) {
    console.warn(
      `\x1b[33m[nereus] Warning: Current platform/architecture (${process.platform}-${process.arch}) is not in the prebuilt binary matrix.\x1b[0m`
    );
    console.warn("[nereus] Please build from source using: cargo install nereus-cli");
    return;
  }

  const target = SUPPORTED_TARGETS[targetKey];
  const binaryDest = path.join(vendorDir, target.bin);

  // If binary already exists and is executable, skip download
  if (fs.existsSync(binaryDest)) {
    console.log(`[nereus] Native binary already installed at ${binaryDest}`);
    return;
  }

  // Check if download is explicitly skipped
  if (process.env.NEREUS_SKIP_DOWNLOAD === "1" || process.env.NEREUS_SKIP_DOWNLOAD === "true") {
    console.log("[nereus] NEREUS_SKIP_DOWNLOAD is set. Skipping download.");
    return;
  }

  const archiveName = `nereus-${TAG}-${targetKey}.${target.ext}`;
  const defaultUrl = `https://github.com/${REPO}/releases/download/${TAG}/${archiveName}`;
  const downloadUrl = process.env.NEREUS_DOWNLOAD_URL || defaultUrl;
  const tempArchive = path.join(vendorDir, archiveName);

  console.log(`[nereus] Fetching Nereus ${TAG} for ${targetKey}...`);
  console.log(`[nereus] Download URL: ${downloadUrl}`);

  try {
    await downloadFile(downloadUrl, tempArchive);
    console.log("[nereus] Download complete. Extracting archive...");
    extractArchive(tempArchive, vendorDir);

    // Clean up temporary archive file
    if (fs.existsSync(tempArchive)) {
      fs.unlinkSync(tempArchive);
    }

    // Ensure executable permissions on Unix systems
    if (process.platform !== "win32" && fs.existsSync(binaryDest)) {
      fs.chmodSync(binaryDest, 0o755);
    }

    console.log(`\x1b[32m[nereus] Successfully installed ${target.bin} to ${vendorDir}\x1b[0m`);
  } catch (err) {
    console.warn(`\x1b[33m[nereus] Could not download prebuilt binary: ${err.message}\x1b[0m`);

    // Clean up partial downloads
    if (fs.existsSync(tempArchive)) {
      try {
        fs.unlinkSync(tempArchive);
      } catch (_) {}
    }

    // Fallback: Check if local workspace binary exists
    if (tryCopyLocalBinary(vendorDir, target.bin)) {
      console.log("\x1b[32m[nereus] Fallback to local development binary succeeded.\x1b[0m");
      return;
    }

    console.warn("\x1b[33m[nereus] Prebuilt binary was not installed.\x1b[0m");
    console.warn("[nereus] You can compile from source by running:");
    console.warn("         cargo build --release -p nereus-cli");
    console.warn(`         cp target/release/${target.bin} ${vendorDir}/`);

    // In CI or production install with release tag, fail if neither downloaded nor locally found
    if (process.env.NEREUS_STRICT_INSTALL === "1") {
      process.exit(1);
    }
  }
}

if (require.main === module) {
  install().catch((err) => {
    console.error(`[nereus] Fatal install error: ${err.message}`);
    process.exit(1);
  });
}

const { resolveBinary } = require("./bin/nereus.js");

module.exports = {
  getTargetKey,
  SUPPORTED_TARGETS,
  downloadFile,
  extractArchive,
  resolveBinary,
};
