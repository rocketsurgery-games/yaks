---
id: yaks-3b02
title: 'Encoding: parallel style grid (baseline) + sparse-background variant'
type: task
priority: 3
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T14:43:38Z'
parent: yaks-9b8d
labels:
- tui
- eval
---

Current approach: pristine char grid, then an aligned style-id grid + legend. Add a sparse variant using space for background to cut noise and chunk better. Cost: forces column-exact cross-reference of two 2D grids (hardest thing for an LLM). Baseline to beat.

---
▸ 2026-08-22T14:43:38Z
Implemented in the headless harness as the 'parallel' encoding (a39e). Kept for back-compat/default; superseded by spans/interleaved per docs/tui-style-eval.md.
