---
id: yaks-d635
title: 'edtui PR: syntect via pure-Rust fancy-regex (drop onig C dep)'
type: feature
priority: 3
created: '2026-08-24T22:19:46Z'
updated: '2026-08-25T03:26:31Z'
parent: yaks-6099
labels:
- ui
- edtui
- upstream
---

edtui depends on syntect with DEFAULT features, which enable default-onig -> onig/onig_sys (Oniguruma C). That forces a C toolchain into any consumer's cross-compile. PR edtui to depend on syntect with default-features=false + features=[default-fancy] (pure Rust), or expose a syntax-highlighting-fancy feature. Unblocks shipping markdown coloring in yaks' pure-Rust release. Requires GitHub fork.

---
▸ 2026-08-25T03:26:31Z
Obviated by yaks-2956: we're dropping edtui's syntect highlighting entirely (coloring both detail + editor with a hand-rolled highlighter via edtui's public state.highlights API), so yaks no longer needs a fancy-regex edtui. Slaughtering. Could still be offered upstream as a courtesy for other edtui users, but that's not yaks work.
