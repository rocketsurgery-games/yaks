---
id: yaks-4e15
title: Human-editing UX + platform legibility tradeoff
type: task
priority: 2
created: '2026-08-31T20:47:23Z'
updated: '2026-08-31T20:47:23Z'
parent: yaks-4fe6
labels:
- git
---

Two UX options once refs are truth. (a) Read-only cache: all mutation via yaks CLI/TUI to ops; humans keep grep/read but lose direct file editing. (b) Read-write working copy (jj-style): yaks diffs the edited file cache against the compiled snapshot and turns changes into ops; files feel authoritative, refs are truth; more magic, needs sync-before-rebuild to avoid clobbering. LEGIBILITY TRADEOFF (a real con): refs hide the herd from the platform: invisible on GitHub web UI, not in git log, not fetched by default, not in PR diffs. Committed .yaks/ files are the opposite: visible on GitHub, in PRs, in git log. So git-store is private/hidden-by-default; file-store is surfaced. Cuts both ways depending on whether you want the herd visible to team/platform.
