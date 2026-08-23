---
id: yaksrs-c913
title: 'Release CI: GitHub Actions workflow to build 5-platform binaries and publish @rocketsurgery/yaks to npm'
type: feature
priority: 1
created: '2026-08-23T14:17:24Z'
updated: '2026-08-23T14:19:13Z'
labels:
- dist
- ci
---

Tag-driven (v*) workflow: matrix builds all 5 target binaries into artifacts/<triple>/yaks, then assembles dist/npm via build-npm.mjs and publishes each package with NPM_TOKEN. workflow_dispatch does a full dry-run (build + assemble + npm publish --dry-run) so the pipeline is testable without publishing or a token.

---
▸ 2026-08-23T14:19:13Z
Done. .github/workflows/release.yml: test gate -> matrix build (aarch64/x86_64 darwin, x86_64/aarch64 linux, x86_64 windows) -> publish. Cross builds are clean (arboard uses pure-Rust x11rb on Linux; only aarch64-linux needs the gcc cross linker). Tag push vX.Y.Z = real publish via NPM_TOKEN + NODE_AUTH_TOKEN; workflow_dispatch = full dry-run (npm publish --dry-run, no token). Enforces tag==Cargo.toml==npm version lockstep before publishing. RELEASING.md updated with the automated path + NPM_TOKEN secret setup. YAML parses; version/lockstep logic verified locally.
