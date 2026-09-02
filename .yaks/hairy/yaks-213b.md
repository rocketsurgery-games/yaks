---
id: yaks-213b
title: 'PR-driven integration: coordinator opens id-free PRs from worker branches'
type: feature
priority: 3
created: '2026-08-30T22:52:32Z'
updated: '2026-08-30T22:52:32Z'
parent: yaks-3901
labels:
- git
---

Adapt the worktree flow to GitHub PRs. Privacy boundary: yak ids may appear in commit messages (powers yaks commits --grep) but NOT in PR titles/descriptions or external trackers; use rollup --keys for upstream links. Prefer coordinator-owns-PRs (workers produce committed branches; coordinator pushes + gh pr create with id-free text) over each worker self-submitting (per-worker gh auth + repeated privacy enforcement). Note: GitHub squash-merge derives the commit message from the id-free PR body, which breaks the --grep provenance join; the git log --follow join over the yak file still works, so keep provenance anchored on the file move, not the message.
