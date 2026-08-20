# Releasing yaks-rs

`yaks` ships as a single static binary. Two distribution paths:

## npm (primary; esbuild/Biome-style)

A thin `yakherder` launcher package declares one prebuilt binary per platform
as `optionalDependencies`; npm installs only the one matching the host's
os/cpu, and `bin/yaks.js` execs it. Users get it via `npm i -g yakherder`
(command stays `yaks`) or `npx yakherder`.

Release steps:
1. Build release binaries for each target into `artifacts/<rust-target-triple>/yaks[.exe]`
   (CI matrix; cross-compile or per-runner). Targets are listed in
   `scripts/build-npm.mjs`.
2. `node scripts/build-npm.mjs <version>` — assembles `dist/npm/*` (launcher +
   per-platform packages, versions matched).
3. Publish: `for d in dist/npm/*/; do (cd "$d" && npm publish --access public); done`

Sanity test the launcher without publishing:
- `node npm/yakherder/test-mapping.mjs` (host→package mapping + deps consistency)
- Place a host binary at `npm/yakherder/node_modules/<host-pkg>/bin/yaks` and run
  `node npm/yakherder/bin/yaks.js list`.

## cargo-dist (GitHub Releases + shell/brew installers)

For tag-driven GitHub Releases with shell/powershell/homebrew installers (and,
optionally, its own npm installer), use cargo-dist ("dist"):
1. `cargo install cargo-dist` (or `dist`).
2. `dist init` — pick targets + installers; this generates/validates
   `dist-workspace.toml` and `.github/workflows/release.yml`.
3. Cut a release by pushing a `v<version>` tag.

Not committed yet (no gh upstream); run `dist init` once the repo is pushed.

## Version bump

Keep `Cargo.toml`, `npm/yakherder/package.json`, and the tag in lockstep.
