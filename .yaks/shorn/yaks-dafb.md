---
id: yaks-dafb
title: 'TUI slice 6b-ii: view-manager picker (v)'
type: task
priority: 4
created: '2026-08-22T02:44:46Z'
updated: '2026-08-22T02:57:19Z'
parent: yaks-86a3
labels:
- rust
- phase2
---

Modal view manager (v): list all views (pinned + not), activate, pin/unpin (keep >=1 pinned), reorder up/down, rename (edtui), delete custom views. Persists immediately via views_store. Builds on 6b-i.

---
▸ 2026-08-22T02:57:19Z
Done. Overlay::ViewPicker(sel) — v opens a modal view manager in the right pane listing ALL views (active ▸, pinned *, count, (builtin) lock). Keys: j/k/Ctrl-N/P navigate; Enter activates (set_view + close); p/Space pin/unpin (guarded by can_unpin so >=1 tab remains, persisted); J/K reorder (move_view, persisted, active view tracked by key so it follows the move); r rename (edtui, seeded with the name, returns to the picker); d delete custom views (built-ins refused), fixing the active index by key; Esc/q close. All mutations persist via views_store. 103 tests (was 97): picker activate/unpin/move/rename/delete + snapshot. Warning-free. Completes slice 6 and Phase 2.
