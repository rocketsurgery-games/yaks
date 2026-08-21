---
id: yaksrs-48c3
title: 'TUI slice 3d: fuzzy task picker + deps (D) + reparent (R)'
type: task
priority: 3
created: '2026-08-21T00:08:31Z'
updated: '2026-08-21T00:08:31Z'
parent: yaksrs-86a3
labels:
- rust
- phase2
---

Add a fuzzy task-picker overlay (floating results list above a filter line, updating as you type; arrows/Ctrl-N/P/Tab cycle, Enter selects) reproducing Python fuzzy_pick_task. Wire D add-dependency (pick target, exclude self + cycles -> Herd::dep_add) and R reparent (pick new parent or clear -> Herd::reparent, guarding cycles). Reload + notification. Snapshot the picker overlay.
