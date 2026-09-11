---
id: yaks-1a30
title: 'Pattern: persistent worktree for a serial multi-phase arc'
type: task
priority: 3
created: '2026-09-11T01:39:46Z'
updated: '2026-09-11T01:51:09Z'
labels:
- meta
- skills
---

yaks-coordinating documents ephemeral per-lane worktrees for PARALLEL fan-out and a human-driven interactive lane, but NOT a long-lived worktree+branch worked by a SEQUENCE of runs across a serial multi-phase arc (e.g. b1cc: 8 sequenced phases). Document the shape: cut once from main HEAD (which holds the committed plan), keep across all phases, remove at arc end; HITL routes through the coordinator-at-the-worktree (no cross-worktree handback, unlike a spawned parallel worker); per-branch-herd implication -- main's herd is stale for in-flight phases unless merged incrementally (the merge-cadence decision); file-tool SOP still applies to any spawned sub-agent (explicit wt/ paths), which the coordinator-drives-directly model avoids. b1cc is the first user + evidence.

---
▸ 2026-09-11T01:51:09Z [coordinator]
Shorn: added the 'Serial-arc run shape (persistent worktree, sequential phases)' section to yaks-coordinating -- cut once/keep/remove-at-end; HITL via coordinator-at-worktree; checkpoint aggressively (squash each phase to main, then merge main back) per Joel's cadence answer (A, esp. team mode); executor choice. b1cc is the first user.
