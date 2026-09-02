---
id: yaks-4e3c
title: 'Modes + sync: solo (local refs) vs team (push/pull refs/yaks/*); refspec friction'
type: task
priority: 2
created: '2026-08-31T20:47:23Z'
updated: '2026-08-31T20:47:23Z'
parent: yaks-4fe6
labels:
- git
---

Solo/local = ops in local refs/yaks/*, never pushed; nothing to gitignore since refs are not working-tree. Team = git-bug native workflow: push/pull the yak refs; concurrent edits auto-merge (op-log CRDT) on fetch, then rebuild cache. Worktrees share one .git, so refs/yaks/* is shared across all worktrees automatically: a live shared herd for free, and isolation-compatible (ref writes to shared .git are allowed even under Claude Code worktree isolation). FRICTION: custom refs need explicit refspec config (remote.origin.fetch/push); git clone/push ignore them by default; teammates must opt in. git-bug also has a Bridge workflow (two-way GitHub/Jira sync), a richer version of our one-way rollup projection.
