---
id: yaksrs-cb67
title: 'Encoding: row-interleaved style grid'
type: task
priority: 3
created: '2026-08-22T13:29:05Z'
updated: '2026-08-22T13:41:45Z'
parent: yaksrs-9b8d
labels:
- tui
- eval
---

Emit each text row immediately followed by its style-id row, so cross-reference distance is one line, not a whole screen. Cheap variant of the parallel grid that may sharply improve column alignment. Test vs baseline.

---
▸ 2026-08-22T13:41:45Z
Dominates the parallel baseline on both axes: cheaper (338 vs 364 tok) and easier cross-reference (style row adjacent to its text). Both 7/7 so far. Parallel (3b02) effectively obsoleted unless a case shows otherwise.
