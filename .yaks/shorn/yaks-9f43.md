---
id: yaks-9f43
title: 'Encoding: frame diffs for multi-step sessions'
type: task
priority: 3
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T14:47:45Z'
parent: yaks-9b8d
labels:
- tui
- eval
---

Encode frame N as a delta from N-1 via Buffer::diff (changed cells / rows only). Large token savings across an agent-driving session; focuses attention on what changed (usually the relevant part). Distinct from single-frame comprehension; pairs with any base encoding.

---
▸ 2026-08-22T14:47:45Z
Done (v1): --diff on the headless harness. After the first (full) frame, emits only changed body lines as 'L<i>: <line>'; header tagged full/diff. Encoding-agnostic (diffs the serialized body, so works with any --style-encoding or plain). Compact for text/overlay changes (e.g. opening a picker = 2 changed lines). KNOWN LIMIT: navigation under a style encoding cascades (per-frame style-id renumbering) -> follow-up yaks-6f87 (stable style-id registry). 94 unit + 19 CLI tests, warning-free.
