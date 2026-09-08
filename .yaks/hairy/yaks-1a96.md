---
id: yaks-1a96
title: 'TUI-snapshot evidence: emit toque LLM-text vs SVG?'
type: idea
priority: 4
created: '2026-09-08T22:56:46Z'
updated: '2026-09-08T22:56:46Z'
parent: yaks-10fb
labels:
- ui,docs
---

Joel's Q (from yaks-52eb): for TUI evidence attached to yaks, should we emit toque's LLM-friendly style-encoded TEXT snapshot (cheaper for agents, greppable, diffable, slightly less human-friendly) instead of / alongside SVG (human-friendly image, but an image in practice — bigger, needs an opener)? Either way: attach as an EXTERNAL file (yaks-52eb decision), never inline. Consider: text snapshot for the coordinator/agent judge + SVG for the human, or one format. Cheap to produce both from the same headless buffer.
