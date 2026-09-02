---
id: yaks-64b3
title: Incremental migration path (dual-write then flip authority)
type: task
priority: 2
created: '2026-08-31T20:47:23Z'
updated: '2026-08-31T20:47:23Z'
parent: yaks-4fe6
labels:
- git
---

De-risk by staging. Phase 0: files authoritative (today). Phase 1: add an op-log in refs that MIRRORS file mutations (dual-write); files still authoritative; validate compile == files. Phase 2: flip: refs authoritative, files become the derived cache; reuse all read code; write path goes through ops. Phase 3: team sync (refspecs) + working-copy editing. Each phase is shippable and reversible, and proves the engine against existing file behavior before we trust it.
