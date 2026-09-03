---
id: yaks-11e9
title: 'a3a6 TUI: inbox as a filter/saved-view; remove the i toggle'
type: feature
priority: 3
created: '2026-09-03T22:28:44Z'
updated: '2026-09-03T22:36:47Z'
parent: yaks-a3a6
labels:
- ui
---

After the prep lands: use FilterSpec's needs predicate for an 'inbox' saved view (like 'recent') and/or a filter usable in any view; remove the App.inbox_only modal toggle from yaks-548b. Lane scope: src/tui.rs + src/tui/*.rs only.

---
▸ 2026-09-03T22:36:47Z [wt-tui]
Shaving. Plan: (1) add unpinned Inbox built-in view (needs_only spec) to default_views; (2) add composable 'inbox' chip to the filter drawer deps row, wiring needs_only into Drawer::build_spec; (3) remove App.inbox_only modal state, toggle_inbox, the rows() special-case, the 'i' keybinding + help entry; (4) add needs_only to filter_summary; (5) update headless tests + regen view_picker/filter_drawer snapshots. Keeping Inbox unpinned to avoid reshaping the tab strip across ~15 full-frame snapshots (which would also drop the herd indicator in herd_indicator_on_tab_row); the drawer chip preserves quick access.
