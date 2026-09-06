---
id: yaks-cada
title: Flush un-applied SOPs into coordinating-yaks + reconcile a412
type: task
priority: 2
created: '2026-09-06T16:25:07Z'
updated: '2026-09-06T16:26:11Z'
parent: yaks-a412
labels:
- skills
---

Coverage check found gaps: crash-recovery procedure (run 4) and human-drift/human-created-yak handling are NOT in coordinating-yaks; a412's body lists done items as 'pending' with SUPERSEDED guidance (.worktrees/ -> wt/, --no-ff -> squash-default). Flush: add crash-recovery + human-drift sections to the skill; add a reconciliation note to a412.

---
▸ 2026-09-06T16:26:10Z [coordinator]
DONE. coordinating-yaks: fixed the stale pre-flight '--no-ff' -> 'squash-merge'; added a Recovery section (lost/crashed worker -> recover from the worktree, don't restart) and a 'Human drift is the human's' paragraph (leave edits untouched; human-created untracked yaks are theirs to introduce). a412 reconciled via note. Skills are now current for the next runs.
