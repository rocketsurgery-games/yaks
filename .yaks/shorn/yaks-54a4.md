---
id: yaks-54a4
title: 'TUI slice 4a: content-filter re-coloring in tree + inline search (/)'
type: task
priority: 3
created: '2026-08-21T01:36:05Z'
updated: '2026-08-21T01:42:51Z'
parent: yaks-86a3
labels:
- rust
- phase2
---

Live filtering over the loaded set via FilterSpec. f opens a filter drawer (toggle type/priority/label/status facets + a text query) with live preview; / opens inline incremental search (updates the query as you type, Enter keeps it, Esc reverts). Esc reverts the live filter to the active view's spec. Both drive filter::apply; the tree keeps ghost-family context but non-matching anchors dim. Add the deferred content-filter re-coloring in tree::build so matches highlight and non-matches dim rather than vanish. Reuse the edtui single-line query. Snapshot the drawer + an active search.

---
▸ 2026-08-21T01:38:07Z
Splitting slice 4: the Python filter drawer is a 7-row chip/text form (status/type/priority/labels/search/parent/deps) — a big piece on its own. 4a does the filter plumbing (tree::build focus/members re-coloring, matching Python build_tree: content matches light up as focus, non-matching ancestors come along dimmed, rest pruned) plus inline incremental search (/). 4b will be the full drawer (f).

---
▸ 2026-08-21T01:42:51Z
Done. tree::build now takes &FilterSpec and applies Python build_tree focus/members logic: with a content filter, universe members matching the content predicate become focus (bright), their ancestors join dimmed to root them, and the rest is pruned; no filter keeps the old anchors=tab / family=ghost behavior. Made FilterSpec::matches pub + added content_active(). App gained a live filter field + clamp_cursor. Inline search overlay (/) reuses edtui single-line, edits filter.search on every keystroke (live re-color), Enter keeps, Esc restores prior query; list-focus Esc clears an active filter. Persistent "filter: ... (Esc clears)" status indicator via filter_summary(). 70 tests (was 66): tree re-color unit test + 3 inline-search tests. Warning-free.
