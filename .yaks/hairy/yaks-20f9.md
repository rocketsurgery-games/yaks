---
id: yaks-20f9
title: b1cc/8 Extract runtime + final reconcile (snapshots, slim tui.rs, docs)
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-11T01:30:07Z'
parent: yaks-b1cc
depends_on:
- yaks-e1d1
labels:
- ui
verify: cargo test -p yaks
---

src/tui/runtime.rs: run/event_loop/setup_watcher/setup/restore. Then reconcile: confirm src/snapshots/ fully migrated to per-module snapshots/ dirs; tui.rs reduced to App struct + Focus/Overlay + mod decls (+ core ctors); update AGENTS.md 'Layout' + any docs referencing tui.rs structure; cargo test --workspace + yaks doctor green; record final line-count before/after as evidence.
