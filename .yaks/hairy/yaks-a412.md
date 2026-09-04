---
id: yaks-a412
title: Skill updates from our decisions + how to VALIDATE skills
type: task
priority: 2
created: '2026-09-03T22:25:55Z'
updated: '2026-09-04T17:32:37Z'
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

---
▸ 2026-09-04T04:18:40Z [coordinator]
FOURTH PARALLEL RUN (reliability lanes) — done, clean, and it survived a HARNESS CRASH mid-run. Lanes: doctor (yaks-39d4, herd.rs/json.rs/main.rs) + skill (yaks-6f21, coordinating-yaks fallback rung + pre-flight checklist). Zero-conflict merge; 224 bin + 21 CLI green.

CRASH-RECOVERY SIGNAL (strong): the harness crashed after the doctor sub-agent had a compiling+passing implementation but BEFORE it committed/sheared. Because the work lived in its worktree, nothing was lost — the coordinator inspected the uncommitted diff, ran tests (224 green), and committed+sheared it. The worktree model is crash-resilient: a lost agent session != lost work. Recovery procedure: git -C <wt> status/diff to assess, cargo test to validate, then coordinator commits+shears. The skill lane had fully committed pre-crash, so it just merged.

DOCTOR = first mechanical validation predicate realized (from this yak's list): 'yaks doctor' checks dup-status-dir (add/add merge hazard) + dangling parent/depends_on, exits non-zero, CI-usable. Ran it on our REAL herd (post ~4 parallel runs / dozen merges): ALL CLEAR — validates both doctor and the disjoint-leaf merge discipline (parallelism never corrupted the herd). Next predicates to mechanize: 'every landed commit names its yak id' and 'main stayed src-clean during a worktree run' (both held every run).

---
▸ 2026-09-04T04:29:31Z [coordinator]
FIFTH RUN — SCALED TO 3 AGENTS (the 'does 3 make sense?' experiment). Three disjoint territories: CLI (yaks-2120: positional title + create --json, main.rs/json.rs) + TUI (yaks-685e: needs-block accent, tui.rs/detail.rs) + skills (yaks-766d: working-a-yak ask guidance). Zero-conflict 3-way merge; 224 lib + 22 cli green; doctor clean.

VERDICT on 3 agents: it works, but the binding constraint is DISJOINT-SCOPE AVAILABILITY + COORDINATOR ATTENTION, not agent count. Finding 3 genuinely independent low-uncertainty territories (CLI/TUI/skills here) is the gate; when they exist, 3 merges as cleanly as 2. Coordination cost (3 prompts to scope, 3 returns to verify, 3 merges) is noticeably higher than 2 but stayed manageable. Sweet spot: 2-3. I'd go to 3 only with clearly independent + low-uncertainty lanes; 4+ would likely strain coordination faster than it adds throughput (avoid unless the work is trivially partitionable, e.g. code + docs + skills + an isolated crate).

FALLBACK-RUNG VALIDATED: 2 of 3 agents hit the edit-tool-refuses-gitignored-.worktrees-path condition and used the terminal fallback; 1 agent's edit tool accepted the worktree path directly. The behavior is INCONSISTENT across agent instances -> the fallback rung (just added to coordinating-yaks) is load-bearing, not optional.

NEW OBSERVATION (TUI worker): this repo's insta snapshots are STYLE-AGNOSTIC (buffer_to_string emits only cell .symbol(), no fg/bg/modifier), so a pure color/bold change produces ZERO snapshot diff -> TUI color work is low snapshot-churn, but color is NOT snapshot-tested (per-cell style capture lives in crates/toque, unused by these tui tests). Worth noting for future TUI-styling yaks.

---
▸ 2026-09-04T17:32:37Z [coordinator]
SIXTH RUN (bulk ops, 2 lanes) + HITL DOGFOODED. Ran a real ask: yaks-7cc8 (filter-mutation safety model) sits in inbox with the needs badge, and I proceeded with the unblocked id-list/multi-select halves — the coordinator-mediated HITL flow worked exactly as the skill prescribes (ask + proceed on unblocked scope; don't block-and-wait). All 3 sub-agents this session used the edit-tool TERMINAL FALLBACK (gitignored .worktrees path refused) — now a near-certainty, confirming the fallback rung is essential. NEW drift pattern observed: the human CREATED a new yak (yaks-d4d3, a p1 eval tool) as an untracked file mid-run; treated it like all human drift — left untouched + flagged, did not commit or build it (their yak to introduce). de85 worker showed strong scope discipline: reverted a status-line tweak that would have rippled a snapshot into tests/snapshots/ (out of scope). Bulk-slaughter-via-selection skips the per-child guard (noted on de85) — a safety follow-up.
