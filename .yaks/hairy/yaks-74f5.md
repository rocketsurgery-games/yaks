---
id: yaks-74f5
title: 'Git merge driver for .yaks/: conflict-free herd merges across branches'
type: feature
priority: 2
created: '2026-08-30T22:52:32Z'
updated: '2026-08-30T22:52:32Z'
parent: yaks-3901
labels:
- git
---

Per-branch worktree herds reconcile at merge. Notes are append-only, so concurrent appends can auto-merge. Add a .gitattributes rule for .yaks/**/*.md plus a merge strategy: built-in merge=union as a cheap first cut (concatenates both sides), then a frontmatter-aware custom driver that unions note blocks (dedupe), takes max(updated), unions labels/depends_on, and on a status/rename conflict (same yak moved to different dirs on two branches) picks furthest-along or flags for yaks doctor. Emerged from the parallel-worktree experiment (yaks-8f81).
