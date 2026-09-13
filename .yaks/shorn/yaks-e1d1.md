---
id: yaks-e1d1
title: 'b1cc/7 Split impl App: key handlers + actions/commits'
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-13T21:07:43Z'
parent: yaks-b1cc
depends_on:
- yaks-74be
labels:
- ui
verify: cargo test -p yaks
---

src/tui/handlers.rs: free fn handle_key + handle_overlay_key/handle_create_key/handle_drawer_key/handle_view_picker_key/handle_help_key/handle_cmdline_key + field_cancel/register_double_esc/request_cancel/run_command/cmd_write/cmd_quit. src/tui/actions.rs: all open_* + commit_* + apply_edit/resolve_pick/resolve_confirm/set_needs_edit/toggle_star/etc. As 'impl App' blocks. Move tests + snapshots. Green.

---
▸ 2026-09-13T21:07:43Z [coordinator]
verify: `cargo test -p yaks` -> PASS (exit 0)

---
▸ 2026-09-13T21:07:43Z [coordinator]
Shorn: split the REST of impl App into two impl App blocks. src/tui/actions.rs (26 methods): view actions + overlay openers (toggle_star/is_starred, save_current_view/persist_views, view picker + reorder/delete, move_cursor, state_header, toggle_collapse/selected, all open_* pickers/forms/drawer/help/attach/comment/ask, edit_target_at/block_starts/jump_block). src/tui/handlers.rs (39 methods + free fn handle_key): handle_create_key, commit_form/create/edit_form, dep/reparent/ref pickers, set_needs_edit, handle_help_key, drawer_live_preview/close_drawer/handle_drawer_key, register_double_esc/request_cancel/form_is_dirty, handle_cmdline_key/run_command/cmd_write/cmd_quit, field_cancel, handle_overlay_key, commit_edit/commit_fuzzy, resolve_pick, apply_edit, resolve_confirm, + top-level handle_key. Both use super::*; methods pub(crate). Made Overlay/PickAction/ConfirmAction pub(crate) (now crossed module boundaries via pub(crate) signatures). tui.rs impl App now only new+with_herd. tui.rs 4521->2604. cargo test -p yaks 242+25 PASS, no warnings, 22 snaps intact. Boundary note: actions vs handlers split at contiguous ranges (openers land in actions even though a few are handler-adjacent) -- pragmatic over per-method sorting.
