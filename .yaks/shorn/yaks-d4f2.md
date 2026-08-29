---
id: yaks-d4f2
title: 'Parity: detail pane mirrors list operations (S/P/T/L/D/R/X/c/C/f/*, J/K, G)'
type: task
priority: 3
created: '2026-08-22T20:24:55Z'
updated: '2026-08-22T20:26:33Z'
parent: yaks-0a93
labels:
- rust
- tui
- parity
---

Python's detail pane mirrors nearly all list mutating ops; Rust's only has E. Add S/P/T/L/D/R/X/*/c/C/f, plus J/K (next/prev task staying in detail) and G (bottom). All act on selected(), which works regardless of focus. Reset detail view state on J/K.

---
▸ 2026-08-22T20:26:33Z
Detail pane now mirrors the list mutating ops: S/P/T/L/D/R/X/E, c/C, f, * — all act on selected(), which is focus-independent. Added J/K (next/prev task without leaving detail; resets per-task view state) and G (scroll last line to viewport bottom). Help Detail section gained J/K. Verified via headless (S opens picker in detail, J advances selection). 110 unit + 19 CLI green.
