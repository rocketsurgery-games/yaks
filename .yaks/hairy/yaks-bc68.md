---
id: yaks-bc68
title: inbox must show every needs-blocked yak (status-independent); ask should guard finished yaks
type: bug
priority: 2
created: '2026-09-03T17:59:04Z'
updated: '2026-09-03T17:59:04Z'
parent: yaks-594b
labels:
- cli
---

FOUND BY DOGFOODING: 'yaks ask yaks-b517' set needs=human on a SHORN yak, but 'yaks inbox' (currently hairy+shaving only) never shows it -> a silent, invisible block. Invariant to enforce: if needs is set, the yak MUST appear in inbox regardless of status. Fix: inbox filters on needs.is_some() across all statuses (or at least not-dead). Separately, 'ask' on a shorn/dead yak is almost always a mistake -> warn (or refuse) when blocking finished work. Decide: is a needs block on a done yak ever meaningful? Likely no.
