---
id: yaks-cc52
title: Following a link to a yak outside any pinned view shows the previous yak
type: bug
priority: 3
created: '2026-09-23T21:40:23Z'
updated: '2026-09-23T21:43:01Z'
labels:
- ui
---

Found by the yaks-158d design scout: follow_link to a yak with no pinned status view (e.g. a dead yak) leaves the cursor unchanged; the detail pane renders rows()[cursor], so it keeps showing the previous yak under a '→ id' notice. yaks-28b4 guards back/forward but not follow_link. yaks-63f3 split goto_task out of follow_link — check whether that path has the same gap. See yaks-158d's design attachment (P3).

---
▸ 2026-09-23T21:43:01Z [coordinator]
Update after merge: yaks-63f3's goto_task (now used by follow_link) checks whether the cursor landed on the target and otherwise shows '<id> isn't shown in any view'. That may already cover this; confirm by following a link to a dead yak before fixing.
