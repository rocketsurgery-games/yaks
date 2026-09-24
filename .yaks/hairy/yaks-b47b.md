---
id: yaks-b47b
title: No default or explicit herd should error in `yaks create`
type: bug
priority: 3
created: '2026-09-23T03:52:41Z'
updated: '2026-09-23T03:52:41Z'
labels:
- cli
- herds
---

This can be quite a weird foot-gun, because you end up falling back to `yak-`, and can end up accidentally creating a new, unintended herd.
