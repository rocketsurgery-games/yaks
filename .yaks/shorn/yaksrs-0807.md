---
id: yaksrs-0807
title: 'Parity: tree / ghost family rendering (investigate vs Python)'
type: task
priority: 3
created: '2026-08-22T03:44:55Z'
updated: '2026-08-22T15:57:53Z'
parent: yaksrs-0a93
labels:
- rust
- tui
- parity
---

Rust shows chevrons + indentation + pulls shorn ghosts (Gamma) with children; Python looked flat for the fixture data. Compare on a herd with clear hairy parent/child chains; decide indentation/chevron/ghost/collapse behavior to match. docs/tui-parity.md #5.

---
▸ 2026-08-22T15:57:53Z
Investigated head-to-head on a purpose-built fixture (hairy parent/child chains + shorn ghost ancestor + shaving ghost descendant), driving both TUIs via the shared headless protocol. Result: tree/ghost/indentation/child-order MATCH on both Hairy and Shorn tabs (tree.rs::build == Python tree.build_tree algorithmically). Rust collapse correctly hides subtree + shows chevron/count; Python collapse can't be verified through the pyte harness (redraw timing artifact) but implements the same apply_collapse logic. Original 'Python looked flat' didn't reproduce -- was against data without parent links. Only diff is the wide-emoji continuation-cell capture artifact (1 vs 2 spaces), not real. No Rust change needed. docs/tui-parity.md #5 updated to [done].
