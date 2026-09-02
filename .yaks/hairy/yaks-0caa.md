---
id: yaks-0caa
title: Provenance rework under a ref store (no file-history join)
type: task
priority: 3
created: '2026-08-31T20:47:23Z'
updated: '2026-08-31T20:47:23Z'
parent: yaks-4fe6
labels:
- git
---

The git log --follow over a yak FILE join dies (no authoritative file; the cache file history is meaningless). Replacements: (a) an op that records the completing commit SHA (explicit, author-attributed link); (b) a bridge/convention mapping yak to commit; (c) keep the yaks commits --grep join by putting yak ids in code commit messages (still works). Net: provenance moves from the yak file moved with the code to an op/commit references the other. Attribution improves (every op has a signed author intrinsically); the code-link needs an explicit mechanism.
