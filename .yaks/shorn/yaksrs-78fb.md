---
id: yaksrs-78fb
title: 'TUI slice 6a: per-user cache + collapsed-state persistence'
type: task
priority: 3
created: '2026-08-21T01:36:15Z'
updated: '2026-08-21T02:02:50Z'
parent: yaksrs-86a3
labels:
- rust
- phase2
---

Persistence + saved queries. v view picker, V save current filter as a named view; * toggle a task into a working-set/pin; show pin + match counts. Persist App.collapsed and saved views/pins per-herd in a per-user cache dir (~/.cache/yaks/<slug>, mirroring Python) — never committed, rebuildable. Load on startup, save on change. Snapshot the view picker + a pinned/counted list.

---
▸ 2026-08-21T02:01:13Z
Splitting slice 6. 6a: per-user cache (~/.cache/yaks/<slug>.json, XDG_CACHE_HOME honored; slug = stable hash of abs herd root — own hash, not sha1, since it is a private rebuildable cache) storing {collapsed:[ids]}. Load on with_herd, save on every collapse toggle. 6b (new): saved views (v/V) + pins/working-set (*) + counts, which are user intent -> config dir, not cache.

---
▸ 2026-08-21T02:02:50Z
Done. New tui/cache.rs: per-user, per-herd rebuildable UI-state cache at $XDG_CACHE_HOME/yaks/<slug>.json (default ~/.cache), slug = FNV-1a of the abs herd root (own hash, not sha1 — private cache). Stores {"collapsed":[ids]}. Herd gained a root() accessor. App::with_herd loads collapsed on open (+clamps cursor); toggle_collapse persists after every change. load_from/save_to are path-pure for testing. 87 tests (was 84): round-trip, missing-file, slug-stability. Warning-free.
