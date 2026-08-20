---
id: yaksrs-ddff
title: 'TUI slice 2: tree (ghost family) + collapse + paging + sparse chrome'
type: task
priority: 2
created: '2026-08-20T23:12:54Z'
updated: '2026-08-20T23:50:02Z'
parent: yaksrs-86a3
labels:
- rust
- ui
---

Port build_tree (no-filter path: anchors=tab status; universe=anchors+ancestors+descendants; ghost=non-anchor family, dimmed) + apply_collapse (hidden counts, chevron). Indented rows with status glyph; Space toggles collapse; d/u half-page, PageUp/Down full. Sparse restyle: drop bordered boxes -> single left divider on detail; no pane titles. Content-filter re-coloring deferred to slice 4.

---
▸ 2026-08-20T23:50:02Z
Done. tui/tree.rs ports build_tree (anchors=tab status; universe=anchors+ancestors+descendants; non-anchor family rendered as dimmed ghosts) + apply_collapse (hidden counts + chevron). List renders indented rows with status glyph + ▾/▸ chevron; Space folds/unfolds; d/u half-page, PageUp/Down full; cursor over visible (post-collapse) rows. Sparse restyle: dropped the bordered boxes for a single left-divider on the detail pane, no pane titles. Content-filter re-coloring (bright matches vs dim context) still deferred to the filter slice. 2 TUI snapshots + tree unit tests; 39 tests, warning-free.
