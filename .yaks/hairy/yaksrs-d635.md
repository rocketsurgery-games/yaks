---
id: yaksrs-d635
title: 'edtui PR: syntect via pure-Rust fancy-regex (drop onig C dep)'
type: feature
priority: 3
created: '2026-08-24T22:19:46Z'
updated: '2026-08-24T22:19:46Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
---

edtui depends on syntect with DEFAULT features, which enable default-onig -> onig/onig_sys (Oniguruma C). That forces a C toolchain into any consumer's cross-compile. PR edtui to depend on syntect with default-features=false + features=[default-fancy] (pure Rust), or expose a syntax-highlighting-fancy feature. Unblocks shipping markdown coloring in yaks' pure-Rust release. Requires GitHub fork.
