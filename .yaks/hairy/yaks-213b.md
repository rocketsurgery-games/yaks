---
id: yaks-213b
title: 'PR-driven integration: coordinator opens id-free PRs from worker branches'
type: feature
priority: 3
created: '2026-08-30T22:52:32Z'
updated: '2026-09-03T22:25:35Z'
parent: yaks-3901
labels:
- git
---

Adapt the worktree flow to GitHub PRs. Privacy boundary: yak ids may appear in commit messages (powers yaks commits --grep) but NOT in PR titles/descriptions or external trackers; use rollup --keys for upstream links. Prefer coordinator-owns-PRs (workers produce committed branches; coordinator pushes + gh pr create with id-free text) over each worker self-submitting (per-worker gh auth + repeated privacy enforcement). Note: GitHub squash-merge derives the commit message from the id-free PR body, which breaks the --grep provenance join; the git log --follow join over the yak file still works, so keep provenance anchored on the file move, not the message.

---
▸ 2026-09-03T22:25:35Z [coordinator]
DECISION (merge vs squash, team mode): the ONLY hard rule is that the yak id must appear in whatever commit(s) land on main — that's what the 'yaks commits' grep-join relies on, and both merge and squash satisfy it (squash does NOT break provenance). Above that rule it's topology taste: --no-ff merges preserve the parallel-lane topology + richer 'yaks commits' --follow history and are PREFERRED in team mode (git history is otherwise redundant with the yak's own notes, which are the authoritative work-trail). A 'squash each lane to one well-messaged commit' hybrid is defensible and loses little. Avoid per-yak cherry-picking across branches (fiddly, fights reconcile-at-merge); to pull main-side updates into a live branch use 'git merge main', all-or-nothing.
