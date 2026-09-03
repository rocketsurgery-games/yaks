---
id: yaks-5ade
title: Update coordinating-yaks skill with validated SOPs (file-tool, disjoint-type, HITL, merge)
type: task
priority: 2
created: '2026-09-03T22:26:25Z'
updated: '2026-09-03T22:27:10Z'
parent: yaks-a412
labels:
- skills
---

Fold the two-run-validated conventions into skills/dev/coordinating-yaks/SKILL.md: file-tool explicit-path SOP (+ nested-worktree-invisible-to-main-status refinement); disjoint scoping is about TYPES not just files (shared-type -> coordinator prep-commit or one lane); HITL routes through the coordinator + fresh spawns (no live cross-worktree feedback, no per-yak cherry-pick); merge-vs-squash rule (id-in-message invariant, --no-ff preferred). Update the attribution note (--as is now BUILT).

---
▸ 2026-09-03T22:27:10Z [coordinator]
DONE. coordinating-yaks SKILL.md updated: (1) file-tool gotcha -> a 3-step SOP + the nested-worktree-invisible-to-main-status refinement (validated across 2 runs, never bit when followed); (2) new 'disjoint scoping is about TYPES not just files' rule w/ the coordinator-prep-commit-or-one-lane remedy + the 'grep TypeName {' pre-scan (caught the FilterSpec hazard in run 2); (3) Attribution section rewritten (--as is BUILT: --as > $YAKS_ACTOR > git user); (4) new Merge/integration section (id-in-message invariant, --no-ff preferred, no per-yak cherry-pick); (5) HITL section: ask/answer/inbox + the across-worktrees-routes-through-coordinator rule (spawn fresh, hand back, don't wait). Each SOP traces to a real run. Feeds the validation-predicate ideas in yaks-a412.
