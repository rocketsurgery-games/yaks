---
id: yaks-20c5
title: 'TUI slice 6b-i: view substrate (views replace tabs) + working-set + save-view + counts'
type: task
priority: 4
created: '2026-08-21T02:01:13Z'
updated: '2026-08-22T02:54:01Z'
parent: yaks-86a3
labels:
- rust
- phase2
---

Saved views (v picker to switch, V save current filter as a named view) and a pinned working-set (* toggles a task in/out), persisted to the config dir (not the rebuildable cache). Show pin markers + match counts. Builds on 6a cache infra + slice-4 filters.

---
▸ 2026-08-22T02:44:46Z
Faithful port of the Python view model. 6b-i (substrate): view.rs (View{name,key,status,builtin,pinned,spec,sort_by,sort_dir,limit}; defaults = 3 status views + Recent(flat, updated desc, limit 50) + Starred/working-set; custom_view). views_store.rs -> config dir ($XDG_CONFIG_HOME/yaks/<slug>/, default ~/.config), views.json + working_set.json, reconcile(overlay vs code built-ins). tree::build refactored to derive status scope from the spec (no more tab param). App swaps tabs/tab for views/view/working_set: tab bar shows PINNED views + counts, Tab/[ ] cycle pinned, switching loads the view spec into the live filter, Esc reverts to it, modified marker *. Flat/sorted + working-set row builders. * toggles star (persist) + star markers. V saves the live filter as a named custom view. 6b-ii (new): the v view-manager picker (activate/pin/reorder/rename/delete).

---
▸ 2026-08-22T02:54:01Z
Done. Ported the Python view substrate. New view.rs (View + SortField/SortDir; defaults = 3 status views + Recent[flat, updated desc, limit 50] + Starred[working-set]; custom_view + short_hash). New views_store.rs -> config dir ($XDG_CONFIG_HOME/yaks/<slug>/, default ~/.config): views.json + working_set.json, hand-serialized via serde_json, reconcile(overlay vs code built-ins: built-in structure from code + overlaid name/pinned, custom rebuilt, unknown built-ins dropped, missing appended), move_view/can_unpin (for 6b-ii). tree::build now derives status scope from the spec (no tab param); FilterSpec derives Clone. App swapped tabs/tab -> views/view/working_set: tab bar shows PINNED views + per-view counts + modified *, Tab/[ ] cycle pinned, set_view loads the view spec into the live filter, Esc reverts. rows() dispatches working-set / flat(sorted) / tree. * stars (persist + ★ marker), V saves the live filter as a named custom view. 97 tests (was 87): views_store reconcile/toggle/move/unpin + App view-switch/recent/star/save/revert + star snapshot. Warning-free.
