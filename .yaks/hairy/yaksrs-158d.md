---
id: yaksrs-158d
title: Global back/forward navigation across view states
type: feature
priority: 3
created: '2026-08-23T02:49:48Z'
updated: '2026-08-23T02:49:48Z'
labels:
- ui
---

Carried over from Python-repo yak-c404. The current i/o history is detail-scoped (visited task ids during drill-down; equivalent to the old shorn yak-2d13). Generalize to ONE global stack recording view switches AND task navigations, restoring exact prior UI state: which view, cursor, filter. True recently-viewed. Research/design before implementing: what counts as a nav event, snapshotting filter/ephemeral-view state, interaction with auto-reload.
