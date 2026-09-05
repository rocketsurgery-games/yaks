---
id: yaks-87ff
title: 'Edit-tool refuses gitignored worktree paths: cause found + candidate fix'
type: task
priority: 2
created: '2026-09-05T02:59:27Z'
updated: '2026-09-05T03:07:59Z'
parent: yaks-8f81
labels:
- agent
---

FINDING (probed empirically): the harness file tools (edit_file/write_file) exclude GIT-IGNORED paths from the project file set. Because .worktrees/ is gitignored, sub-agents' file tools refuse worktree paths ('path not found') and must fall back to terminal edits. Proven: my write_file SUCCEEDS on a NON-gitignored worktree path (wt-probe/) and the path is not ignored; it fails on .worktrees/ (gitignored). So the gitignore is the cause, not worktree-ness per se.

IMPACT: reliability LOW (contained by SOP; 6/6 runs clean), correctness LOW (terminal replacements riskier but guarded by match-count asserts + the post-edit 'git -C main status' check catches the bare-path-hits-main hazard), token-spend MODERATE (diagnose-refusal + verbose anchored terminal edits + verification ~= 1.3-2x on edit-heavy tasks; also loses the edit tool's fuzzy/batch affordances). Likely Zed-specific symptom (gitignore-filtered project file set); harnesses editing by absolute path w/o gitignore filtering (Claude Code, aider) probably don't hit it, though agent+worktree support is broadly immature.

CANDIDATE FIX: put worktrees in a NON-gitignored in-project dir (e.g. wt/ instead of .worktrees/). Tradeoff: main 'git status' shows '?? wt/' (cosmetic; we always stage explicit paths, and the main-clean guard is already 'no src/ changes'). MUST hold: sibling/outside-project worktrees are NOT viable (the terminal tool requires cwd inside a project root). NEXT: validate that SUB-AGENT file tools (not just the coordinator's) accept a non-gitignored worktree path in a live run; if yes, drop the terminal-fallback tax going forward.
