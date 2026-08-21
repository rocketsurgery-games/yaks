---
id: yaksrs-78fb
title: 'TUI slice 6: saved views + pins + counts + collapsed persistence (per-user cache)'
type: task
priority: 3
created: '2026-08-21T01:36:15Z'
updated: '2026-08-21T01:36:15Z'
parent: yaksrs-86a3
labels:
- rust
- phase2
---

Persistence + saved queries. v view picker, V save current filter as a named view; * toggle a task into a working-set/pin; show pin + match counts. Persist App.collapsed and saved views/pins per-herd in a per-user cache dir (~/.cache/yaks/<slug>, mirroring Python) — never committed, rebuildable. Load on startup, save on change. Snapshot the view picker + a pinned/counted list.
