---
id: yaksrs-a083
title: 'Parity: review remaining overlays (E/L/X// /D/R/v) vs Python'
type: task
priority: 4
created: '2026-08-22T03:44:55Z'
updated: '2026-08-22T16:03:56Z'
parent: yaksrs-0a93
labels:
- rust
- tui
- parity
---

Capture + compare edit, labels, slaughter confirm, inline search, fuzzy dep/reparent, view picker; extend docs/tui-parity.md #12. Single-key pickers S/P/T already match.

---
▸ 2026-08-22T16:03:56Z
Reviewed all remaining overlays head-to-head (E/L/X// /D/R/v) on a fixture herd via the shared headless protocol. Findings + parity doc #12 updated. Fixes made: fuzzy dep/reparent picker candidates now use status emoji (was [H]/[S]/[N] bracket letters); view picker uses 📌 pinned / 🔒 builtin (was */(builtin) text) -- both consistent with f2cf. Matches: L, / (live inline search), X (reworded Slaughter vs Delete, kept). Acceptable divergences: right-pane placement (D/R/v per d416), R direct-picker+clear-parent-row vs Python p/u prompt. Real gap: E edits body-only vs Python's full form -> filed yaksrs-3b22 (shared create/edit form). 100 unit + 19 CLI green, warning-free.
