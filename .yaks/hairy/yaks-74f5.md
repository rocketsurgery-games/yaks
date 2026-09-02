---
id: yaks-74f5
title: 'Git merge driver for .yaks/: conflict-free herd merges across branches'
type: feature
priority: 2
created: '2026-08-30T22:52:32Z'
updated: '2026-09-02T23:34:26Z'
parent: yaks-3901
labels:
- git
---

Per-branch worktree herds reconcile at merge. Notes are append-only, so concurrent appends can auto-merge. Add a .gitattributes rule for .yaks/**/*.md plus a merge strategy: built-in merge=union as a cheap first cut (concatenates both sides), then a frontmatter-aware custom driver that unions note blocks (dedupe), takes max(updated), unions labels/depends_on, and on a status/rename conflict (same yak moved to different dirs on two branches) picks furthest-along or flags for yaks doctor. Emerged from the parallel-worktree experiment (yaks-8f81).

---
▸ 2026-09-02T23:34:26Z
[coordinator] Evidence from the first real parallel run (yaks-3901): two disjoint-scope workers each shearing only their own leaf yak merged to main with ZERO conflicts, including a genuine 3-way merge. So under the disjoint-leaf convention the driver is not needed. Keep this yak as a safety net for the UNHAPPY path only (workers forced onto the same parent/shared yak); build it only if the deliberate collision test shows the manual reconciliation is painful.
