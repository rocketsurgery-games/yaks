---
id: yaksrs-79c3
title: Reconstruct the yak-tracker SKILL (inline projection rationale + tracker-derived labels)
type: task
priority: 3
created: '2026-08-23T02:04:42Z'
updated: '2026-08-23T02:07:04Z'
parent: yaksrs-1d54
labels:
- skills
---

Adapt yak-tracker: inline the projection rationale (no docs/design/projection.md in yaks-rs), add guidance to label yaks by tracker name (jira/github/linear) per yak-efb9.

---
▸ 2026-08-23T02:07:04Z
Wrote skills/yak-tracker/SKILL.md. Changes vs Python: inlined the projection rationale (many→few, yak→external only, tracker-unaware) since yaks-rs has no docs/design/projection.md. ADDED step 3 'Label by tracker' (jira/github/linear) per yak-efb9, cross-referencing the yak skill's label guidance. Kept hard rules, rollup/import/outbound sections, per-tracker read hints. Verified rollup --keys exists.
