---
id: yaksrs-6f87
title: Stable style-id registry across frames (compact diffs)
type: task
priority: 3
created: '2026-08-22T14:47:25Z'
updated: '2026-08-22T14:58:44Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Frame-diff (9f43) re-emits many lines on navigation because spans/parallel style-ids are assigned per-frame in first-appearance order: moving the selection reassigns ids and cascades changes across rows + the legend. Persist a StyleKey->id registry on the headless Driver (append-only across frames) so ids stay stable; then a cursor move diffs to just the two affected rows. Plain-grid (no style) diffs are already compact. Low-risk enhancement to the token-savings value of frame-diff.

---
▸ 2026-08-22T14:58:44Z
Done: persistent StyleRegistry on the headless Driver assigns stable base36 ids across frames (append-only). Legend lists only ids used in the current frame. Navigation under a style encoding now diffs to just the affected rows (+ the legend only when the used-style set changes) instead of cascading the whole screen. Unit tests: id stability + compact spans diff.
