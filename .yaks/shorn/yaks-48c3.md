---
id: yaks-48c3
title: 'TUI slice 3d: fuzzy task picker + deps (D) + reparent (R)'
type: task
priority: 3
created: '2026-08-21T00:08:31Z'
updated: '2026-08-21T00:57:55Z'
parent: yaks-86a3
labels:
- rust
- phase2
---

Add a fuzzy task-picker overlay (floating results list above a filter line, updating as you type; arrows/Ctrl-N/P/Tab cycle, Enter selects) reproducing Python fuzzy_pick_task. Wire D add-dependency (pick target, exclude self + cycles -> Herd::dep_add) and R reparent (pick new parent or clear -> Herd::reparent, guarding cycles). Reload + notification. Snapshot the picker overlay.

---
▸ 2026-08-21T00:57:55Z
Done. Added Overlay::Fuzzy(FuzzyPick): a filter-as-you-type task picker reusing edtui (single-line query) with a ranked results list in the right pane and the query on the status line. Ranking mirrors Python fuzzy_pick_task (id-prefix < id-substr < title-substr, then priority, then id; empty query lists all, capped 20). Wired D add-dependency (excludes self, existing deps, and cycle-forming targets via new filter::depends_on_transitively) and R reparent (excludes self + descendants + current parent; offers a clear-parent row when the task has a parent, via allow_none). Nav: Up/Down/Tab/Ctrl-P/Ctrl-N; Enter commit; Esc cancel; other keys edit the query and reset selection. Cycle-prevention kept in the TUI layer (façade/CLI untouched). 66 tests (was 59): filter unit test + 2 fuzzy render snapshots + 4 live dep/reparent tests. Warning-free. Completes slice 3 (3a/3b/3d; 3c merged into 3b).
