---
id: yaks-b1cc
title: Split src/tui.rs monolith into a render/ module tree
type: task
priority: 3
created: '2026-09-10T23:45:35Z'
updated: '2026-09-10T23:45:58Z'
labels:
- meta
- ui
---

src/tui.rs is ~7K lines / 270KB, funnelling nearly all UI work through one file and defeating file-disjoint parallel scoping (see the coordinating-skill yak). Splitting render_* and the handle_*_key handlers into a render/ (and handlers/) module tree would give future parallel batches real file-level disjointness. Corollary of dogfooding the UI batch.
