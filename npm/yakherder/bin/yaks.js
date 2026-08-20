#!/usr/bin/env node
"use strict";
// Thin launcher: pick the prebuilt `yaks` binary for the host platform and
// exec it. The binaries ship as per-platform packages declared in this
// package's optionalDependencies (npm installs only the one matching the
// host's os/cpu). This is the esbuild/Biome distribution pattern.
const { spawnSync } = require("node:child_process");

// host key ("<platform> <arch>") -> platform package name
const TARGETS = {
  "darwin arm64": "yakherder-darwin-arm64",
  "darwin x64": "yakherder-darwin-x64",
  "linux x64": "yakherder-linux-x64",
  "linux arm64": "yakherder-linux-arm64",
  "win32 x64": "yakherder-win32-x64",
};

function pkgFor(platform, arch) {
  return TARGETS[`${platform} ${arch}`] || null;
}

function binPath() {
  const pkg = pkgFor(process.platform, process.arch);
  if (!pkg) {
    throw new Error(
      `yaks: unsupported platform ${process.platform}/${process.arch}. ` +
        `Install a prebuilt binary from GitHub Releases instead.`
    );
  }
  const exe = process.platform === "win32" ? "yaks.exe" : "yaks";
  try {
    return require.resolve(`${pkg}/bin/${exe}`);
  } catch (_e) {
    throw new Error(
      `yaks: platform package "${pkg}" is not installed. ` +
        `Reinstall with optionalDependencies enabled (npm install --include=optional).`
    );
  }
}

module.exports = { TARGETS, pkgFor, binPath };

if (require.main === module) {
  let bin;
  try {
    bin = binPath();
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
  const res = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
  if (res.error) {
    console.error(`yaks: failed to launch binary: ${res.error.message}`);
    process.exit(1);
  }
  process.exit(res.status === null ? 1 : res.status);
}
