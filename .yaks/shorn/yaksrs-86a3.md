---
id: yaksrs-86a3
title: 'Phase 2: TUI (ratatui + crossterm + edtui)'
type: task
priority: 3
created: '2026-08-20T03:25:50Z'
updated: '2026-08-22T02:57:35Z'
labels:
- rust
- phase2
---

Rebuild the TUI immediate-mode. Board/detail/tabs, modal keymaps, embedded edtui editor (vim/emacs/nano profiles), repaint-free rendering. Children to be decomposed when Phase 1 nears done.

---
▸ 2026-08-22T02:57:35Z
Phase 2 complete. All slices shorn: 1 (skeleton), 2 (tree+collapse+sparse), 3a/3b/3d (mutations: state/priority/type/slaughter, shared edtui editor for labels/create/body, fuzzy deps+reparent), 4a/4b (filter re-coloring+inline search, filter drawer), 5a/5b (structured navigable detail+jumplist, detail find), 6a (collapsed cache), 6b-i/6b-ii (view substrate + view-manager picker). TUI is a thin layer over the print-free Herd facade with a pure render(). 103 tests, warning-free.
