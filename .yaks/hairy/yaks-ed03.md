---
id: yaks-ed03
title: Cache = the current .yaks/ file layout, materialized (reuse read/TUI code)
type: task
priority: 1
created: '2026-08-31T20:47:23Z'
updated: '2026-08-31T20:47:23Z'
parent: yaks-4fe6
labels:
- git
---

KEY UNLOCK. Make the derived cache BE today's on-disk layout: compile each yak snapshot and serialize it to the same markdown+frontmatter file under hairy/shaving/shorn/dead. Then every existing read path (list/show/search/log/rollup/refs and the whole TUI-over-files) works UNCHANGED against the cache; only the write path becomes op-append + rebuild. Keeps local greppability/diffability. Cache is per-user, rebuildable, stat/generation-validated (see the rkyv index idea yaks-8f50). Invariant preserved: files are a derived index, never a second source of truth. Risk: a human editing the cached files expects persistence, but a rebuild clobbers them (see the human-editing UX yak).
