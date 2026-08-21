---
id: yaksrs-f6ac
title: 'TUI slice 5b: detail-pane incremental search'
type: task
priority: 4
created: '2026-08-21T01:49:29Z'
updated: '2026-08-21T01:59:50Z'
parent: yaksrs-86a3
labels:
- rust
- phase2
---

Incremental search within the detail pane body (/ while focused on detail): highlight matches, n/N to cycle, scroll to match. Builds on the structured detail from 5a.

---
▸ 2026-08-21T01:59:50Z
Done. Overlay::DetailFind(SearchBox) — / in detail focus opens an incremental find over the built detail lines; edits App.detail_find live, scrolls to the first match; Enter keeps, Esc restores the prior query. n/N cycle matches (detail_find_jump, scrolling each into view). Refactored render_dline to a per-char style model (base -> link -> find-match, each overriding) coalesced into spans, so match highlights (current = green, others = yellow) compose cleanly over link highlights. detail_scan() finds case-insensitive occurrences as (line,col,len). 84 tests (was 81): match/cycle + esc-restore behavior + a find snapshot. Warning-free.
