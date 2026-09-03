---
id: yaks-11e9
title: 'a3a6 TUI: inbox as a filter/saved-view; remove the i toggle'
type: feature
priority: 3
created: '2026-09-03T22:28:44Z'
updated: '2026-09-03T22:43:11Z'
parent: yaks-a3a6
labels:
- ui
---

After the prep lands: use FilterSpec's needs predicate for an 'inbox' saved view (like 'recent') and/or a filter usable in any view; remove the App.inbox_only modal toggle from yaks-548b. Lane scope: src/tui.rs + src/tui/*.rs only.

---
▸ 2026-09-03T22:36:47Z [wt-tui]
Shaving. Plan: (1) add unpinned Inbox built-in view (needs_only spec) to default_views; (2) add composable 'inbox' chip to the filter drawer deps row, wiring needs_only into Drawer::build_spec; (3) remove App.inbox_only modal state, toggle_inbox, the rows() special-case, the 'i' keybinding + help entry; (4) add needs_only to filter_summary; (5) update headless tests + regen view_picker/filter_drawer snapshots. Keeping Inbox unpinned to avoid reshaping the tab strip across ~15 full-frame snapshots (which would also drop the herd indicator in herd_indicator_on_tab_row); the drawer chip preserves quick access.

---
▸ 2026-09-03T22:43:07Z [wt-tui]
Done. Removed the modal inbox: deleted App.inbox_only, App::toggle_inbox, the rows() special-case, the 'i' keybinding, and its help entry. Inbox now surfaced two composable ways off the shared needs_only predicate: (1) a 'needs'->'inbox' chip in the filter drawer's deps row (Drawer.needs wired through from_filter/toggle_chip/clear/build_spec/render_drawer), so inbox composes with any status/scope; (2) an unpinned 'Inbox' built-in view in default_views (needs_only spec, flat, sorted updated-desc), reachable via the view picker. Also added needs_only to filter_summary and made view_count flat-aware so the Inbox tab/picker shows the awaiting-a-human count (not the herd size); Recent's count is unchanged (4 in sample) so no extra snapshot churn. Kept the 'a' ask/answer keys and the ⏳ needs badge from yaks-548b. Chose unpinned to avoid reshaping the tab strip across ~15 full-frame snapshots (which would also drop the herd indicator in herd_indicator_on_tab_row); pinning is a one-line bool if Joel wants a tab. Evidence: cargo build --release OK; cargo test --workspace --release = toque 13 + yaks 221 + cli 20 + 1 doctest, all pass. Tests: rewrote inbox test as inbox_view_lists_needs_blocked_across_statuses (order-insensitive, +count assert), added drawer_inbox_chip_composes_with_status_scope, updated view counts/indices (6 default views now), added 'inbox' to the views_store reconcile test, regenerated view_picker + filter_drawer snapshots. Scope: only src/tui.rs, src/tui/view.rs, src/tui/views_store.rs, 2 snapshots.
