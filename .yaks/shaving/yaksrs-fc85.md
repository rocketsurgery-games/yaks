---
id: yaksrs-fc85
title: Vi editing ergonomics
type: feature
priority: 2
created: '2026-08-26T13:01:50Z'
updated: '2026-08-26T13:14:01Z'
labels:
- ui
---

Umbrella for making the embedded editors behave the way vi users expect. Groups the follow-on ergonomics ideas surfaced while building the structured description/comment editing (yaksrs-127e).

Core tension: Esc currently cancels the whole edit, which defeats modal (Normal-mode) editing in the fields. The fix is to give a non-Esc cancel path (double-Esc / Ctrl-C), then let Esc mean 'to Normal mode' in vi. Children cover the enabler, the modal defaults, the cancel-safety, and the :command affordances.
