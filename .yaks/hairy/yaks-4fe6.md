---
id: yaks-4fe6
title: 'Research: git as a yak store (git-bug-style op-log in refs) vs filesystem'
type: idea
priority: 2
created: '2026-08-31T20:46:52Z'
updated: '2026-08-31T20:48:17Z'
labels:
- git
---

Could yaks move from file-authoritative storage to a git-bug-style operation-log in a dedicated ref namespace (refs/yaks/*), with today's on-disk .yaks/ layout demoted to a derived, rebuildable cache? git-bug stores entities as ordered CRDT operations in git objects (blob=OperationPack, tree, commit-chain per entity under refs/<ns>/<id>), merged conflict-free via Lamport clocks + DAG ordering, with intrinsic signed authorship. This dissolves the per-branch/merge/PR pain (refs shared across worktrees, isolation-compatible, never in PR diffs, mergeless) at the cost of opacity + a real engine. Key hypothesis: a materialized cache reproducing today's file layout keeps greppability and reuses ~all existing read/query/TUI code, so only the WRITE path changes. Compared against Model 3 (coordinator-sole-writer on filesystem), the sanest pure-FS option. Children track each dimension; findings preserved as notes.

---
▸ 2026-08-31T20:48:17Z
Storage spectrum (synthesis). There are not two options but a spectrum, trading legibility for mergelessness: (1) files on code branches (today): visible on the platform + in PRs, merges WITH code, PR-pollution. (2) files on a separate orphan ref (e.g. refs/yaks/herd), materialized into .yaks/ locally: off code branches, no PR-pollution, shared via .git, KEEPS files/grep, but still needs file-merge handling for concurrent same-yak edits (union driver + disjoint assignment) and a checkout/cache step. Cheap: no engine. (3) op-log in refs (git-bug): off-branch, mergeless (CRDT), shared, intrinsic attribution, but a real engine + platform-invisible + opacity-unless-cached. Key point: option 2 decouples 'herd off code branches' (cheap, keeps files) from 'mergeless op-log' (expensive). Model 3 (coordinator-sole-writer) is orthogonal: it removes concurrency by construction and works with ANY of the three loci.
