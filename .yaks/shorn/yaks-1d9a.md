---
id: yaks-1d9a
title: cargo-dist + npm installer skeleton
type: task
priority: 3
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T19:42:08Z'
parent: yaks-26c0
labels:
- rust
---

---
▸ 2026-08-20T19:41:05Z
Tooling: gh + npm/node present; cargo-dist + hyperfine not installed. Scope: build the real, node-testable npm launcher (Biome/esbuild-style: thin main package + per-platform binaries as optionalDependencies, bin shim execs the host's binary) + an assembler script + RELEASING.md. cargo-dist (GitHub Release/brew/shell installers) documented as a  step rather than a possibly-wrong committed config.

---
▸ 2026-08-20T19:42:08Z
Done. npm/yakherder: launcher package (per-platform binaries as optionalDependencies + Node bin shim that execs the host binary; esbuild/Biome pattern; command stays yaks). scripts/build-npm.mjs assembles dist/npm/* (launcher + platform packages) from artifacts/<triple>/yaks. RELEASING.md documents the npm flow plus the cargo-dist (dist init) path for GitHub Releases/brew/shell installers (deferred until the gh upstream exists). Verified with node: mapping unit test passes; e2e smoke with a stand-in platform package had the launcher exec the real binary (yaks stats, exit 0). gitignore now excludes dist/ node_modules/ artifacts/.
