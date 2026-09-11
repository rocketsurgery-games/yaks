---
id: yaks-e1d1
title: 'b1cc/7 Split impl App: key handlers + actions/commits'
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-11T01:30:07Z'
parent: yaks-b1cc
depends_on:
- yaks-74be
labels:
- ui
verify: cargo test -p yaks
---

src/tui/handlers.rs: free fn handle_key + handle_overlay_key/handle_create_key/handle_drawer_key/handle_view_picker_key/handle_help_key/handle_cmdline_key + field_cancel/register_double_esc/request_cancel/run_command/cmd_write/cmd_quit. src/tui/actions.rs: all open_* + commit_* + apply_edit/resolve_pick/resolve_confirm/set_needs_edit/toggle_star/etc. As 'impl App' blocks. Move tests + snapshots. Green.
