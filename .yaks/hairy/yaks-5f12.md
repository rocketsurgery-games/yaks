---
id: yaks-5f12
title: 'Decision: git-store vs Model 3 (coordinator-sole-writer on FS)'
type: task
priority: 1
created: '2026-08-31T20:47:23Z'
updated: '2026-09-01T23:31:59Z'
parent: yaks-4fe6
labels:
- git
---

Model 3 (pure FS): files authoritative, coordinator is sole herd writer on main; zero new engine; no merge/concurrency (single writer); provenance via file+commit (works); human-editable files; but limited worker autonomy, no live cross-worktree sharing, PR-cleanliness needs the coordinator to land herd separately. Git-store: refs authoritative + materialized cache; big new engine (ops, clocks, compile, sync); mergeless + live-shared-across-worktrees + PR-clean + intrinsic attribution + multi-writer-safe + isolation-compatible; but ref-sync friction, platform-invisibility, provenance rework, ongoing maintenance (git-bug itself is barely maintained, a signal of cost/thin adoption). Lean: Model 3 now (tiny effort, max reliability for small careful parallelism); git-store as the north-star architecture if multi-agent-native coordination justifies the engine, pursued via the incremental path so the file experience survives as the cache.

---
▸ 2026-09-01T23:31:59Z
Refined architectural judgment (path-independent). Retracting the 'git-store as north star' framing. Divorced from what we have built: FILE-BASED is the better TARGET for yaks' core identity (tasks as plain greppable/diffable/hand-editable files; status as directory; single simple binary). git-store trades that differentiator away for mergeless multi-writer concurrency, which careful parallelism does not need (disjoint assignment / single-writer removes contention with zero engine). Even WITH a materialized cache, git-store still loses platform visibility (GitHub/PR/git log), trivial hand-editing, and file-level git history. git-store wins ONLY in the private + distributed + offline + multi-writer/multi-machine quadrant (git-bug's niche). Not a general north star; a different product that wins in one quadrant.
