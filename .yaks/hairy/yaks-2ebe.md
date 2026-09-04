---
id: yaks-2ebe
title: 'Write side: bulk and filter-driven mutations'
type: feature
priority: 2
created: '2026-08-30T19:34:21Z'
updated: '2026-09-04T17:32:37Z'
parent: yaks-3901
labels:
- cli
---

Bulk label and metadata edits by id-list or dynamic filter. Bulk state transitions. Filter-driven mutations with a --dry-run guardrail. Umbrella over yaks-5fae, yaks-8d53, yaks-9ccc, yaks-de85.

---
▸ 2026-09-04T17:32:37Z [coordinator]
Bulk mutation by explicit ID-LIST landed (yaks-5fae): 'yaks update a b c ...' and 'yaks reparent a b c --parent P' apply one edit across many ids (per-id result, exit non-zero on any miss, single-id unchanged). TUI multi-select landed (yaks-de85): 'm' marks/unmarks the cursor yak (Space=collapse and v=view were taken; v-vs-view rebind deferred), a '●' gutter marker, and a selection-aware 'S' that bulk-transitions the marked set via existing herd.transition. Both via a 2-lane parallel run; 226 bin + 23 cli green; doctor clean. FILTER-DRIVEN mutation is the only piece left and is BLOCKED on the human decision yaks-7cc8 (in inbox).
