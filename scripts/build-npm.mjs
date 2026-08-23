#!/usr/bin/env node
// Assemble npm packages from prebuilt binaries.
//
// Expects binaries under artifacts/<rust-target-triple>/yaks[.exe] and emits
// dist/npm/yaks/ (the launcher) + dist/npm/yaks-<npm-target>/ (one per
// platform, each carrying its binary). Dist dir names are flat so the publish
// glob works; the package names inside are scoped (@rocketsurgery/yaks[-*]).
// Publish each dist/npm/* with `npm publish`. Usage: node scripts/build-npm.mjs <version>
import { existsSync, mkdirSync, copyFileSync, readFileSync, writeFileSync, chmodSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const version = process.argv[2];
if (!version) {
  console.error("usage: node scripts/build-npm.mjs <version>");
  process.exit(2);
}

// rust target triple -> { npm target suffix, os, cpu, exe }
const MAP = {
  "aarch64-apple-darwin": { t: "darwin-arm64", os: "darwin", cpu: "arm64", exe: "yaks" },
  "x86_64-apple-darwin": { t: "darwin-x64", os: "darwin", cpu: "x64", exe: "yaks" },
  "x86_64-unknown-linux-gnu": { t: "linux-x64", os: "linux", cpu: "x64", exe: "yaks" },
  "aarch64-unknown-linux-gnu": { t: "linux-arm64", os: "linux", cpu: "arm64", exe: "yaks" },
  "x86_64-pc-windows-msvc": { t: "win32-x64", os: "win32", cpu: "x64", exe: "yaks.exe" },
};

const out = join(root, "dist", "npm");
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

const optionalDeps = {};
const template = readFileSync(join(root, "npm", "platform-template", "package.json"), "utf8");

for (const [triple, m] of Object.entries(MAP)) {
  const src = join(root, "artifacts", triple, m.exe);
  const pkgName = `@rocketsurgery/yaks-${m.t}`;
  const dirName = `yaks-${m.t}`; // flat dist dir; scoped name lives in package.json
  optionalDeps[pkgName] = version;
  if (!existsSync(src)) {
    console.warn(`skip ${pkgName}: missing ${src}`);
    continue;
  }
  const pkgDir = join(out, dirName);
  mkdirSync(join(pkgDir, "bin"), { recursive: true });
  const manifest = template
    .replaceAll("__TARGET__", m.t)
    .replace('"__OS__"', JSON.stringify(m.os))
    .replace('"__CPU__"', JSON.stringify(m.cpu))
    .replace('"0.0.0"', JSON.stringify(version));
  writeFileSync(join(pkgDir, "package.json"), manifest);
  const dest = join(pkgDir, "bin", m.exe);
  copyFileSync(src, dest);
  chmodSync(dest, 0o755);
  console.log(`built ${pkgName}@${version}`);
}

// launcher package with version-matched optionalDependencies
const launcher = JSON.parse(readFileSync(join(root, "npm", "yaks", "package.json"), "utf8"));
launcher.version = version;
launcher.optionalDependencies = optionalDeps;
const ldir = join(out, "yaks");
mkdirSync(join(ldir, "bin"), { recursive: true });
writeFileSync(join(ldir, "package.json"), JSON.stringify(launcher, null, 2) + "\n");
copyFileSync(join(root, "npm", "yaks", "bin", "yaks.js"), join(ldir, "bin", "yaks.js"));
console.log(`built launcher @rocketsurgery/yaks@${version}`);
console.log(`\nPublish: for d in dist/npm/*/; do (cd "$d" && npm publish --access public); done`);
