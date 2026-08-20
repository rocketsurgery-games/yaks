---
id: yaksrs-4e7f
title: 'Status moves: shave/shorn/regrow/slaughter/revive (atomic rename)'
type: task
priority: 2
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T03:56:15Z'
parent: yaksrs-6e21
depends_on:
- yaksrs-6b8c
labels:
- rust
---

---
▸ 2026-08-20T03:54:34Z
Matching Python model.move_task: find current file by exact path (O(1) per status dir), no-op if already at dest, else rename into dest dir + rewrite with bumped updated (now_iso). No commit-stamping (the CLI does not stamp commit). Commands shave/shorn/regrow/slaughter/revive with Python-identical messages ('X is already ...' / 'Shorn! X' etc.) + old-name aliases (work/close/reopen). Moving now_iso into store for reuse. Adding fs-level move tests.

---
▸ 2026-08-20T03:56:15Z
Done. store::move_task (rename into dest dir + rewrite with bumped updated; no-op if already there; NotFound handling) + find_task_file (O(1) exact-path probes) + now_iso moved to store. main: shave/shorn/regrow/slaughter/revive with Python-identical messages + aliases work/close/reopen (and ready for next). 6 tests pass (incl. 2 fs-level move tests). About to dogfood: shorn THIS yak with the Rust binary.
