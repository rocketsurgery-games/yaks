---
id: yaks-6f21
title: 'coordinating-yaks: file-tool fallback rung + coordinator pre-flight checklist'
type: task
priority: 3
created: '2026-09-04T04:01:50Z'
updated: '2026-09-04T04:01:50Z'
parent: yaks-a412
labels:
- skills
---

Fold run-3 findings into skills/dev/coordinating-yaks/SKILL.md: (1) file-tool SOP FALLBACK RUNG — if the harness edit tool refuses the gitignored .worktrees/ path, do edits through the terminal with cwd in the worktree (anchored replacements); main stayed src-clean either way across 3 runs. (2) A short coordinator PRE-FLIGHT checklist distilled from the runs: scan shared types ('grep Type {') before fanning out; shared-type change -> prep commit or one lane; spawn workers fresh from main; assign disjoint file scopes; expect + leave human .yaks drift untouched. Keep it tight — this is the accreted-from-runs section.
