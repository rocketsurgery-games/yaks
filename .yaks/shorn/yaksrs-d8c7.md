---
id: yaksrs-d8c7
title: Markdown syntax coloring in the editor (local)
type: feature
priority: 2
created: '2026-08-24T22:11:50Z'
updated: '2026-08-24T22:19:37Z'
parent: yaksrs-6099
labels:
- ui
- edtui
---

Enable edtui's syntax-highlighting feature and attach a SyntaxHighlighter for markdown to the multiline editors (comment + create/edit description). Colorization only (syntect) — not rich bold/heading rendering, which is out of edtui's scope. No fork/PR needed; pure yaks-side config. Watch the dep weight (syntect). Consider making it configurable (herd config vim_mode already exists as a precedent).

---
▸ 2026-08-24T22:19:11Z
Delivered as an OPT-IN cargo feature 'md-syntax' (default OFF). Enabling edtui's syntax-highlighting pulls syntect with default features -> Oniguruma C dep (onig_sys). Cargo feature unification means yaks can't disable edtui's transitively-enabled onig, and the release matrix (aarch64/x86_64 linux-gnu, macOS, windows-msvc) is intentionally pure-Rust/C-free ('all deps are pure Rust' per release.yml). So shipping it on by default would add a C toolchain hazard to cross builds. Implementation: apply_syntax() cfg-gated helper wraps the multiline EditorViews (comment + create/edit description) with a markdown SyntaxHighlighter; config key editor_syntax (theme name; 'off' disables; default base16-ocean-dark) on store::Config + App. Foreground-only coloring, so no background clash. Verified: default build has 0 syntect/onig deps; --features md-syntax builds + tests pass. Build locally with . Making it default-on is blocked on edtui using pure-Rust fancy-regex (spun out).

---
▸ 2026-08-24T22:19:37Z
(correction to prior note: build locally with: cargo run --features md-syntax)
