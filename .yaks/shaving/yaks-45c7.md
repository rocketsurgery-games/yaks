---
id: yaks-45c7
title: 'CLI: --needs filter + a needs marker on list/next rows'
type: feature
priority: 3
created: '2026-09-03T17:59:19Z'
updated: '2026-09-03T20:01:32Z'
parent: yaks-594b
labels:
- cli
---

Two query gaps: (1) fmt_row shows [glyph] id pN type title [labels] with no indication a yak is needs-blocked -> a blocked yak looks identical to a ready one in 'list'. Add a marker (e.g. a ⚠ or 'needs:<who>' tag). (2) There is no general '--needs [who]' filter flag; 'inbox' is sugar over it. Add --needs to the shared FilterFlags so list/log/search can select blocked yaks; inbox becomes 'list --needs' with the status-independence from yaks-bc68.

---
▸ 2026-09-03T19:58:25Z [coordinator]
[coordinator] DESCOPE for the parallel run: drop the '--needs' FilterSpec filter half. FilterSpec has EXHAUSTIVE literal constructions in tui/ (tui.rs:427, tui/views_store.rs:118), so adding a FilterSpec field would force edits in the TUI lane's files -> not disjoint. Keep 45c7 to the high-value ROW MARKER in fmt_row (main.rs only). '--needs' filter is deferred (it overlaps 'inbox' already); revisit as its own yak, likely paired with a coordinator prep commit that adds the FilterSpec field + fixes all constructions up front.
