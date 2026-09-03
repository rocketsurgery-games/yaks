---
id: yaks-f6f8
title: 'a3a6 PREP: add needs predicate to FilterSpec + matches (coordinator prep commit)'
type: task
priority: 3
created: '2026-09-03T22:28:44Z'
updated: '2026-09-03T22:28:51Z'
parent: yaks-a3a6
labels:
- cli
---

Shared-type change done up front on main (per the coordinating-yaks disjoint-TYPE rule) so the CLI + TUI wiring can then fan out cleanly. Add a needs filter to FilterSpec + filter::matches, and fix every exhaustive construction.
