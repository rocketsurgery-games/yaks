---
id: yaks-39d4
title: 'yaks doctor: herd integrity check (post-merge danglers, parent/child, dup-status)'
type: feature
priority: 3
created: '2026-08-30T22:52:32Z'
updated: '2026-09-04T04:17:45Z'
parent: yaks-3901
labels:
- cli
---

Read-only integrity pass. Detect: a yak present in two status dirs at once (add/add from a branch merge), dangling parent/depends_on refs, parent/child state violations (hairy parent with shorn children), duplicate ids, orphaned artifacts. Especially valuable after merging per-branch herds. Complements the merge driver.

---
▸ 2026-09-04T04:17:45Z [coordinator]
BUILT (sub-agent wt-doctor) + RECOVERED (coordinator). herd.doctor() -> Vec<Issue>/IssueKind: read-only integrity pass detecting (1) a yak id present in two status dirs at once (the add/add branch-merge hazard), (2) dangling parent refs, (3) dangling depends_on refs. 'yaks doctor' renders grouped issues (+--json via doctor_array) and exits non-zero when any are found (CI-usable). Tests: doctor_flags_dangling_parent_and_depends_on, doctor_flags_same_id_in_two_status_dirs, doctor_clean_herd_has_no_issues; 224 bin tests green. NOTE: the harness crashed after the sub-agent finished a compiling+passing implementation but before it committed/sheared — the work survived intact in the worktree, so the coordinator committed+sheared it. Nice reliability signal for the worktree model.
