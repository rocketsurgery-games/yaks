---
id: yaks-0a87
title: 'Checkpoint footgun: commit human drift before squash-merge; never reset --hard before it succeeds'
type: task
priority: 2
created: '2026-09-11T03:50:27Z'
updated: '2026-09-13T21:18:58Z'
labels:
- meta
- skills
---

Live incident during b1cc/5: the serial-arc checkpoint (git merge --squash <branch> then git -C <worktree> reset --hard main) went wrong because (1) an UNCOMMITTED human answer sat on main's .yaks/hairy/yaks-2301.md while the branch had MOVED that file hairy->shorn -> merge --squash hit a conflict and left a HALF-STAGED index with an unmerged (UD) file, so the following '&& git commit' silently no-opped (unmerged files block commit); and (2) the unconditional 'reset --hard main' then moved the branch ref OFF the phase-5 commit (recoverable only via reflog / the commit SHA). Recovered by folding the human notes into the shorn file, git rm-ing the hairy conflict, committing, and re-verifying main green. LESSONS for yaks-1a30 (serial-arc pattern): (a) before ANY squash-merge checkpoint, COMMIT (or stash) all uncommitted human/.yaks drift on main first -- especially answers to files the branch moves; (b) GATE the branch re-sync on checkpoint success -- only reset --hard after the merge+commit actually landed (check exit status / new HEAD), never unconditionally; (c) a conflicted merge --squash is a half-applied state, not a clean abort -- resolve or 'git reset --hard' deliberately. Amend the serial-arc worktree pattern doc accordingly.

---
▸ 2026-09-13T21:18:58Z [coordinator]
RECURRED at b1cc/8 (worse): the 'git add <list>' where one pathspec did NOT exist (.yaks/shaving/yaks-20f9.md, a transient state) made git add ABORT and stage NOTHING (same root cause as the 6c93 slip) -- so the branch commit captured only the PREVIOUSLY-staged snapshot renames, silently dropping the code edits (tui.rs slim, runtime.rs, tests.rs, AGENTS.md). The reset --hard then reverted tui.rs and left main BROKEN (snaps moved, tests still inline). Recovered because untracked files (runtime.rs/tests.rs/shorn/*) survive reset --hard. SHARPER LESSONS: (1) never 'git add' a list that may contain a non-existent path -- it aborts the whole add; add only paths you've confirmed exist, or add per-file. (2) GATE on COMMIT CONTENTS, not just 'HEAD moved' -- verify 'git show --stat' includes every expected file before reset --hard. (3) prefer 'yaks shorn' then stage via 'git add -A -- .yaks/<id>*' patterns that match the actual move, not hardcoded shaving/ paths.
