---
id: yaks-74be
title: 'b1cc/6 Split impl App: view-model + detail navigation'
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-13T20:56:40Z'
parent: yaks-b1cc
depends_on:
- yaks-2301
labels:
- ui
verify: cargo test -p yaks
---

src/tui/viewmodel.rs: rows/flat_rows/working_set_rows/view_count/blocked_ids/pinned_indices/selected/selected_id/task/clamp_cursor/set_view/switch_tab/reload*/active_view. src/tui/detail_nav.rs: detail_dlines/jumps/move_detail_line/jump_link/follow_link/selection+yank/detail_find_*/open_task_in_detail/nav_back/forward/scroll_line_into_view. Both as 'impl App' blocks. Move tests + snapshots. Green.

---
▸ 2026-09-13T20:56:40Z [coordinator]
verify: `cargo test -p yaks` -> PASS (exit 0)

---
▸ 2026-09-13T20:56:40Z [coordinator]
Shorn: split impl App by moving two CONTIGUOUS method clusters into impl App blocks in new modules. src/tui/viewmodel.rs (20 methods): save_ui_state/herd-scope, reload/reload_preserving_selection, active_view/view_count, blocked_ids/pinned_indices, working_set_rows/flat_rows/rows, clamp_cursor, selected/selected_id/task, set_view/switch_tab, is_view_modified/revert_filter_to_view. src/tui/detail_nav.rs (23 methods): select_task/expand_ancestors, detail_dlines/jumps/line_count, move_detail_line/detail_line_to, jump_link, visual selection (toggle_visual/extend_selection/selection_range/yank_selection), scroll_line_into_view, detail_find_*, open_task_in_detail, follow_link, copy_selected_id, detail_next_task, nav_back/forward, select_id. Both use 'use super::*' to inherit tui.rs's scope; methods bulk-marked pub(crate). Compiled + passed FIRST TRY. tui.rs 5068->4521. cargo test -p yaks 242+25 PASS, no warnings, 22 snaps intact.
