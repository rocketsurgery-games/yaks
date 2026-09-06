---
id: yaks-91a8
title: Document yaks bulk (filter-mutation, dry-run-default) in README + yak skill
type: task
priority: 3
created: '2026-09-06T01:55:43Z'
updated: '2026-09-06T01:58:10Z'
parent: yaks-a412
labels:
- docs
---

c411 deliberately skipped 'yaks bulk' because it was landing in parallel; it's merged now. Document it: filter selects + distinct --set-*/--add-label mutate, DRY-RUN by default, --commit to apply, refuses unfiltered. Additive to README + skills/yak/SKILL.md.

---
▸ 2026-09-06T01:55:43Z [coordinator]
[assignment] owner: wt-bulkdocs | scope: skills/yak/SKILL.md + README.md | task: document 'yaks bulk' (filter-select + --set-*/--add-label mutate, dry-run default, --commit, refuses unfiltered). one commit; move shaving->shorn at the end.

---
▸ 2026-09-06T01:58:03Z [wt-bulkdocs]
[evidence] Documented 'yaks bulk' additively in README.md (command-table row + explanatory paragraph) and skills/yak/SKILL.md (command-table row + new 'Bulk field edits' section). Verified flags against 'yaks bulk --help': filters --status/--type/--priority/--label/--search/--ready/--tangled/--needs/--parent-of select; mutations --add-label/--remove-label/--set-priority/--set-type/--reparent/--unparent apply; dry-run by default, --commit to apply; refuses without >=1 filter and >=1 mutation flag; field edits + reparent only (no state transitions).
