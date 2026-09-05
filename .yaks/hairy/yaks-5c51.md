---
id: yaks-5c51
title: Apply the slaughter child-guard to bulk/multi-select slaughter
type: bug
priority: 2
created: '2026-09-05T03:18:30Z'
updated: '2026-09-05T03:18:30Z'
parent: yaks-2ebe
labels:
- ui
---

Safety follow-up flagged in yaks-de85: single-yak slaughter (open_slaughter_confirm) refuses a yak with children ('slaughter them first'), but the multi-select bulk path (PickAction::BulkState 'x') loops herd.transition to Dead WITHOUT that guard. Apply the same per-child guard to the bulk slaughter path (skip + report ids that have children).
