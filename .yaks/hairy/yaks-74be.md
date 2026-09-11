---
id: yaks-74be
title: 'b1cc/6 Split impl App: view-model + detail navigation'
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-11T01:30:07Z'
parent: yaks-b1cc
depends_on:
- yaks-2301
labels:
- ui
verify: cargo test -p yaks
---

src/tui/viewmodel.rs: rows/flat_rows/working_set_rows/view_count/blocked_ids/pinned_indices/selected/selected_id/task/clamp_cursor/set_view/switch_tab/reload*/active_view. src/tui/detail_nav.rs: detail_dlines/jumps/move_detail_line/jump_link/follow_link/selection+yank/detail_find_*/open_task_in_detail/nav_back/forward/scroll_line_into_view. Both as 'impl App' blocks. Move tests + snapshots. Green.
