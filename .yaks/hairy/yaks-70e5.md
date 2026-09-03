---
id: yaks-70e5
title: 'Candidate primitive: yaks diff <refA> <refB> (ref-generic herd diff)'
type: feature
priority: 3
created: '2026-09-03T22:25:35Z'
updated: '2026-09-03T22:25:35Z'
parent: yaks-3901
labels:
- cli
---

Framed explicitly as REF-GENERIC, not worktree-aware (we deliberately reject worktree-awareness as a first-class concept — it violates files-authoritative/portable/degrade-gracefully). 'Show a worktree's yak-state vs main' is just a special case of 'diff the herd between two git refs', which works uniformly for branches/tags/PRs/worktrees.

Today 'git diff main -- .yaks/' already does ~80%. BUILD THIS ONLY IF that raw diff proves too coarse in practice (start by just using it). If built: a read-only 'yaks diff <refA> <refB>' that shells git to enumerate .yaks/ changes between the refs and renders them as YAK-LEVEL deltas: added yaks, status transitions (hairy->shorn etc. from the dir move), and note deltas. Pure read over files-at-two-refs; no worktree semantics anywhere. TUI at most gets a 'compare against ref' view, also ref-generic. Provenance-adjacent to yaks-2610.
