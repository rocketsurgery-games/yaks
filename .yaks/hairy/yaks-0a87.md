---
id: yaks-0a87
title: 'Checkpoint footgun: commit human drift before squash-merge; never reset --hard before it succeeds'
type: task
priority: 2
created: '2026-09-11T03:50:27Z'
updated: '2026-09-11T03:50:27Z'
labels:
- meta
- skills
---

Live incident during b1cc/5: the serial-arc checkpoint (git merge --squash <branch> then git -C <worktree> reset --hard main) went wrong because (1) an UNCOMMITTED human answer sat on main's .yaks/hairy/yaks-2301.md while the branch had MOVED that file hairy->shorn -> merge --squash hit a conflict and left a HALF-STAGED index with an unmerged (UD) file, so the following '&& git commit' silently no-opped (unmerged files block commit); and (2) the unconditional 'reset --hard main' then moved the branch ref OFF the phase-5 commit (recoverable only via reflog / the commit SHA). Recovered by folding the human notes into the shorn file, git rm-ing the hairy conflict, committing, and re-verifying main green. LESSONS for yaks-1a30 (serial-arc pattern): (a) before ANY squash-merge checkpoint, COMMIT (or stash) all uncommitted human/.yaks drift on main first -- especially answers to files the branch moves; (b) GATE the branch re-sync on checkpoint success -- only reset --hard after the merge+commit actually landed (check exit status / new HEAD), never unconditionally; (c) a conflicted merge --squash is a half-applied state, not a clean abort -- resolve or 'git reset --hard' deliberately. Amend the serial-arc worktree pattern doc accordingly.
