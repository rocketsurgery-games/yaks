---
id: yaks-a0a8
title: Cut v0.0.8 release (tui.rs split, detail sticky header, toque SVG frames)
type: task
priority: 2
created: '2026-09-13T22:39:42Z'
updated: '2026-09-13T23:44:36Z'
labels:
- release
verify: cargo test --workspace
---

Cut v0.0.8. Ships since v0.0.7: the yaks-b1cc tui.rs split (6.7k-line monolith -> focused src/tui/ submodules: render, editor, drawer, create, fuzzy, viewmodel, detail_nav, actions, handlers, runtime, test_support, tests), a pinned sticky header in the detail pane when scrolled (yaks-fd5e), detail find that keeps the cursor on the match at Enter and restores it on Esc (yaks-26f0), selection that follows a resort + detail pane closing when the yak drops out of the list (yaks-f207), the ephemeral status line dropping below the tabs when it would overlap (yaks-7a1c), and toque retiring text style-encodings in favor of SVG frame rendering -- which removes the 'yaks tui --style/--style-encoding' flags. Ritual (RELEASING.md): bump Cargo.toml + npm/yaks/package.json to 0.0.8 (lockstep enforced by release.yml), refresh Cargo.lock, commit, tag v0.0.8, push main + tag -> release.yml matrix-builds 5 binaries and publishes the 6 npm packages via OIDC.

---
▸ 2026-09-13T23:43:57Z [Joel Webber]
verify: `cargo test --workspace` -> PASS (exit 0)

---
▸ 2026-09-13T23:44:32Z [Joel Webber]
Version lockstep set to 0.0.8: Cargo.toml + npm/yaks/package.json (the two files release.yml's lockstep check compares against the tag); Cargo.lock refreshed by 'cargo build --release', which now reports 'Compiling yaks v0.0.8' and './target/release/yaks --version' prints 'yaks 0.0.8'. Levers: 'yaks verify yaks-a0a8' (cargo test --workspace) PASS -- 242 unit + 25 cli + 7 toque + 1 doctest, 0 failed, 1 ignored (the on-demand docshots generator). 'node npm/yaks/test-mapping.mjs' ok (launcher host->package mapping + optionalDependencies consistent). 'yaks doctor' all clear. Note: toque stays at 0.1.0 -- release.yml publishes only the six npm packages, not the crate.
