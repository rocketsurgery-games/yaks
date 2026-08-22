---
id: yaksrs-6f87
title: Stable style-id registry across frames (compact diffs)
type: task
priority: 3
created: '2026-08-22T14:47:25Z'
updated: '2026-08-22T14:47:25Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Frame-diff (9f43) re-emits many lines on navigation because spans/parallel style-ids are assigned per-frame in first-appearance order: moving the selection reassigns ids and cascades changes across rows + the legend. Persist a StyleKey->id registry on the headless Driver (append-only across frames) so ids stay stable; then a cursor move diffs to just the two affected rows. Plain-grid (no style) diffs are already compact. Low-risk enhancement to the token-savings value of frame-diff.
