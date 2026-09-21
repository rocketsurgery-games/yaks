---
id: yaks-98d5
title: 'per-herd config: defaults + verify by prefix'
type: feature
priority: 3
created: '2026-09-21T17:32:32Z'
updated: '2026-09-21T17:32:32Z'
parent: yaks-15f7
labels:
- cli
---

Deferrable. Config is single-valued today (one default_type and default_priority, one verify map resolved by label). A farm spanning projects with different levers eventually wants per-prefix config: default type and priority per herd and verify resolution keyed by herd as well as label. Incremental; label-based verify plus per-yak source already cover most of the gap, so this lands after the MVP proves out.
