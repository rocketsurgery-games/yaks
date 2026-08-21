---
id: yaksrs-2a25
title: 'TUI slice 5: detail-pane links/jumplist + detail search + visual-yank'
type: task
priority: 3
created: '2026-08-21T01:36:15Z'
updated: '2026-08-21T01:36:15Z'
parent: yaksrs-86a3
labels:
- rust
- phase2
---

Richer detail pane. Parse task-id references and URLs in the detail body/fields into a navigable link set; a jumplist lets you tab through them and Enter jumps (selecting that task / opening the URL). Detail-pane incremental search (/ within detail) to find text in a long body. Visual-yank: select a range in the detail view and copy it (clipboard via the clipboard primitive when available; otherwise a stub). Keep render pure. Snapshot detail with highlighted links + a jumplist state.
