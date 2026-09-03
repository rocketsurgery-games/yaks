---
id: yaks-a412
title: Skill updates from our decisions + how to VALIDATE skills
type: task
priority: 2
created: '2026-09-03T22:25:55Z'
updated: '2026-09-03T22:45:51Z'
parent: yaks-3901
labels:
- skills
- agent
---

Rolling capture of skill changes our decisions imply, plus the harder question Joel flagged: how do we know a skill is actually working?

PENDING SKILL UPDATES:
coordinating-yaks:
- File-tool explicit-path SOP (PROMOTE from caveat to standard): edit worktree files via the full .worktrees/<name>/ path; after each edit check 'git -C <main> status'. Refinement from run 2: a linked worktree's .yaks/ is INVISIBLE to main status (git skips nested worktrees), so the guard reduces to 'no src/ in main'. Verified across two runs — the gotcha never bit when the SOP was followed.
- Disjoint scoping is about TYPES, not just files: shared-type edits (FilterSpec, Task) collide even across disjoint files (exhaustive literal constructions). Rule: a shared-type change goes in a coordinator PREP COMMIT on main first, or stays within ONE lane — never split across parallel lanes.
- HITL over worktrees routes through the COORDINATOR + fresh spawns; workers hand back (yaks ask on their leaf + return message), never block-and-wait; no per-yak cherry-picking. Live cross-worktree human->worker feedback is explicitly a non-goal.
- Merge vs squash: id-in-message is the invariant; --no-ff preferred in team mode (see yaks-213b).
- Attribution: --as / $YAKS_ACTOR on notes across agents; git author covers committed transitions.
working-a-yak:
- needs/ask/answer: on a human decision, 'yaks ask' + hand back; don't clear your own block.
yak:
- create requires --title (friction until yaks-2120); note it.

HOW TO VALIDATE SKILLS (ideas to develop):
1. The yak+git trail IS the eval substrate. Because agents record evidence/notes and commit with ids, adherence is auditable post-hoc from the herd + diff alone.
2. Mechanical tripwire PREDICATES (prefer these — cheap, deterministic): every landed commit names its yak id (grep); no shear without a preceding evidence note (parse notes vs the shorn transition); main stayed src-clean during a worktree run; each worker touched only its assigned scope (git diff --stat per branch); human drift left untouched. These could live in a 'yaks doctor'-adjacent check or CI.
3. Ablation / adversarial: deliberately omit a rule, see if the failure mode reappears (we implicitly did this with the FilterSpec type-hazard). Controlled A/B: same scoped task with vs without a rule.
4. LLM-as-judge over the trail as a fallback where predicates are too rigid — but bias toward predicates.
5. The method we've been using IS the validation loop: run real parallel rounds, observe deviations, encode each observed success/failure back into the skill AS A PREDICATE. Skills improve by accreting tripwires from real runs. This yak tracks that accretion.

---
▸ 2026-09-03T22:45:51Z [coordinator]
SKILL-SOP REFINEMENT from run 3 (pending fold into coordinating-yaks): the file-tool SOP's 'edit via explicit .worktrees/<name>/ path' can FAIL when the harness's edit tool refuses GITIGNORED paths (.worktrees/ is gitignored) — the CLI worker's edit_file couldn't resolve the worktree path and fell back to TERMINAL-based edits (cwd=worktree, anchored replacements). Both workers stayed main-src-clean regardless. So the SOP needs a fallback rung: if the file tool won't write the gitignored worktree path, do edits through the terminal with cwd in the worktree. New validation predicate candidate: 'main tree src-clean throughout a worktree run' held for a 3rd time -> strong signal the isolation discipline is real. Also: a human's dirty working-tree yak (b517: wat:+needs:human) correctly surfaced in inbox and was left untouched by both workers + coordinator — the 'leave human drift' + 'files-are-authoritative' conventions held again.
