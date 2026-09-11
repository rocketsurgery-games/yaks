---
id: yaks-13f3
title: 'Coordinating skill: scope by function/region when one file is unavoidable'
type: task
priority: 3
created: '2026-09-10T23:45:35Z'
updated: '2026-09-11T01:01:55Z'
labels:
- meta
- skills
---

Recon for the parallel UI batch (yaks-7a1c/yaks-26f0/yaks-f207) hit the wall that src/tui.rs is a 7K-line monolith holding nearly every render fn and key handler, so the yaks-coordinating SOP of disjoint FILES is impossible. Add guidance: when one giant file is unavoidable, scope lanes by function/region within it; run the shared-type scan first to find the collision point -- here App has exactly ONE literal construction (App::new), so any lane adding an App field collides there; prefer yaks needing no new shared-type fields; and anchor each lane's new tests at a distinct line range so the shared test module merges cleanly.

---
▸ 2026-09-11T00:11:13Z [coordinator]
Dogfood run (3 parallel UI lanes; 26f0/f207 script-sheared+merged, 7a1c merged+awaiting visual) surfaced yaks-coordinating additions: (a) SIBLING-SNAPSHOT RIPPLE -- a render change can invalidate an unrelated-looking committed .snap (7a1c changed starred_marker_and_tab_bar.snap); the evidence contract should pre-warn lanes to re-accept AND eyeball such ripples. (b) LINE-ANCHOR DRIFT -- briefs used tui.rs line numbers, but a lane cut AFTER siblings merged saw them shifted ~20 lines (apply_edit 3149->3168); prefer SYMBOL/grep anchors over line numbers. (c) ASK/ANSWER x WORKTREE -- the human's f207 answer landed as UNCOMMITTED drift on main; the coordinator MUST commit the answer before cutting the worker's worktree (worktrees only see committed state). (d) CROSS-WORKTREE REVIEW-ASK -- a lane's 'yaks ask' is recorded on its BRANCH and only reaches main's inbox after merge, so a visual-review ask that gates the shear requires merging/propagating it to main before the human can 'yaks answer'.

---
▸ 2026-09-11T01:01:55Z [coordinator]
Shorn: added to yaks-coordinating an architecture-NEUTRAL 'scope by function when one file is unavoidable' subsection (prefer no-shared-type-change yaks; anchor briefs by symbol not line number; pre-warn snapshot ripple), a matching pre-flight bullet, and the two worktree x ask gotchas (commit the human answer before cutting the worktree; a worker's review-ask only reaches main's inbox after merge). Per the thesis agreed with Joel: teach the coordination technique; keep the split-your-files decision with the project, not skill doctrine. Evidence: edits in skills/dev/yaks-coordinating/SKILL.md.
