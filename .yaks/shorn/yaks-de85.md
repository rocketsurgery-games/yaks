---
id: yaks-de85
title: Multi-select list items for bulk mutations
type: feature
priority: 3
created: '2026-08-23T02:49:48Z'
updated: '2026-09-04T17:28:05Z'
labels:
- ui
---

For mutations where it makes sense -- eg, set state, slaughter, adding a label, perhaps removing common labels, etc -- it would be useful to be able to multi-select list items with 'v' / 'j/k' before an action.

---
▸ 2026-08-26T13:17:57Z
Note: if we want to use vi-style 'v' for this, we'll have to rethink the [v]iew shortcut affordances.

---
▸ 2026-09-04T17:28:05Z [wt-tui]
Shipped v1 in src/tui.rs. KEY BINDING: bound 'm' (mark/unmark cursor row) as the multi-select toggle -- Space was NOT free (it is collapse/expand) and vi-style 'v' collides with the [v]iew picker. The v-vs-view rebinding is DEFERRED (not resolved here), per the original note. Selection state: added 'selected: HashSet<String>' on App, toggled by 'm'; a filled dot (U+25CF, green-bold) renders in the list gutter, taking precedence over the blocked '*'. BULK ACTION: 'S' is now selection-aware -- with marks it opens a bulk h/s/n/x state picker (PickAction::BulkState) that LOOPS the marked ids through the existing herd.transition(), clears the selection, and reloads; with no marks it is the unchanged single-yak picker. This covers bulk shorn AND bulk slaughter. No herd.rs/store.rs changes. Tests: mark_toggles_selection_and_renders_gutter_dot (render) + live::bulk_state_transition_shorns_the_marked_set. Snapshots: only src/snapshots/help_overlay.snap regenerated (added 'm' + 'S (marked)' help entries); reverted the status-line help_hint tweak to avoid touching out-of-scope tests/snapshots. Deferred: mirror 'm' into the detail pane; per-child guard on bulk slaughter.
