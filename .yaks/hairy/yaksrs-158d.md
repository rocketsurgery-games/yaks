---
id: yaksrs-158d
title: Global back/forward navigation across view states
type: feature
priority: 3
created: '2026-08-23T02:49:48Z'
updated: '2026-08-26T13:15:18Z'
labels:
- ui
- search
---

Generalize the (shorn) detail-scoped nav stack from yak-2d13 into a global back/forward history spanning ALL view/UI states, not just yak-detail drill-down. Today nav_history/nav_pos is reset on every _enter_detail and _detail_next_task, so it only tracks navigation within a single detail context.

Goal: one navigation stack recording view switches AND task navigations, so back/forward returns you to the exact prior UI state (which view, cursor, filter). This is the true 'recently viewed' affordance deferred from yak-597c: it covers the 'looked at but did not change' case that the Recent view (derived from updated:) deliberately does not.

Many moving parts: what counts as a nav event; how filter/ephemeral-view state is snapshotted and restored; persistence across sessions; interaction with the 500ms auto-reload. Break into research + design before implementing.

Adjacent: yak-6f33 (carry search context globally) and yak-2d13 (shorn predecessor: detail-scoped nav stack).
