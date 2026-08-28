---
id: yaksrs-5fae
title: Bulk and filtered field updates & reparents
type: feature
priority: 2
created: '2026-08-23T03:43:29Z'
updated: '2026-08-28T04:20:51Z'
parent: yaksrs-8d53
labels:
- cli
---

update and reparent accept multiple ids, and optionally a filter selector to apply across a matching set (e.g. bulk relabel every yak with a given label, or reparent a whole subtree). This is the core moving-and-refactoring affordance. Needs a design decision on how to express the selector safely.
