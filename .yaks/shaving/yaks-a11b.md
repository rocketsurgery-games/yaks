---
id: yaks-a11b
title: 'yaks bulk: filter-driven field mutation (dry-run-default)'
type: feature
priority: 2
created: '2026-09-05T03:18:30Z'
updated: '2026-09-05T03:21:11Z'
parent: yaks-2ebe
labels:
- cli
---

Implements the filter half of bulk mutation per the yaks-7cc8 decision (DRY-RUN BY DEFAULT). New 'yaks bulk' subcommand: FilterFlags select the set, distinct mutation flags apply. Dedicated command (not --flags on update) because FilterFlags --priority/--type collide with update's set-priority/set-type. Field mutations only (labels/priority/type/reparent) — NO state transitions (filter-slaughter deferred as the scariest footgun).
