---
id: yaksrs-fa64
title: 'Encoding: sparse run-list (relational, not grid)'
type: task
priority: 3
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T13:29:05Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Pristine char grid + a list of styled runs as (row, col-span, style); unlisted = background. Turns 2D cross-reference into coordinate arithmetic and drops a whole grid. Hypothesis: strong accuracy / token tradeoff by avoiding column counting.
