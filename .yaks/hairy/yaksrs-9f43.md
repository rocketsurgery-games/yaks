---
id: yaksrs-9f43
title: 'Encoding: frame diffs for multi-step sessions'
type: task
priority: 3
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T13:29:05Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Encode frame N as a delta from N-1 via Buffer::diff (changed cells / rows only). Large token savings across an agent-driving session; focuses attention on what changed (usually the relevant part). Distinct from single-frame comprehension; pairs with any base encoding.
