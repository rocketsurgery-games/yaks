---
id: yaks-d4d3
title: No-yak-ids validation tool
type: feature
priority: 1
created: '2026-09-04T12:10:00Z'
updated: '2026-09-05T03:02:15Z'
labels:
- eval
---

CLI tool for scanning arbitrary text for valid yak-ids, using the same validation mechanism used by the rendering code to highlight yak-links.

To be used by a pre-commit check in private-mode herds, to ensure that yak-ids aren't leaking into the repo. Similar affordance for upstream PRs and issues.
