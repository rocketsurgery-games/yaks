---
id: yaks-26f0
title: Search in detail not sticky
type: bug
priority: 3
created: '2026-08-28T05:02:53Z'
updated: '2026-08-28T05:02:53Z'
labels:
- ui
---

When searching for text within a yak detail, it correctly cycles through all the results with n/N. But when you try to Enter out of this mode, the cursor jumps back to wherever it was before the search. I think it's ok for Esc to drop you back to the old location like vi does. But "enter" after selecting one should leave the cursor on the match.
