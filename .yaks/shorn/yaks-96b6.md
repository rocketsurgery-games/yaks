---
id: yaks-96b6
title: b1cc/3 Extract drawer + create/edit form -> src/tui/{drawer,create}.rs
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-11T03:04:36Z'
parent: yaks-b1cc
depends_on:
- yaks-7483
labels:
- ui
verify: cargo test -p yaks
---

drawer.rs: Drawer + consts + text_field/multiline_field/toggle + impl Drawer + render_drawer/render_chip_row. create.rs: ContentBlock/CreateForm + kind_index/pri_index + impl CreateForm + render_create/render_content_stack/render_block_separator/render_text_row. Move tests + snapshots. Green.

---
▸ 2026-09-11T02:27:45Z [coordinator]
Progress (drawer half): moved Drawer struct + impl Drawer + render_drawer into src/tui/drawer.rs (pub(crate)). Shared form helpers (text_field, multiline_field, toggle, *_CHOICES, DRAWER_ROWS, DRAWER_LABEL_W, render_chip_row, render_text_row, status_word) stay in tui.rs and are reached via super:: (create form uses them too); App drawer key-handlers stay in tui.rs (access Drawer via pub(crate)). Decision (per the phase-2 convention, recorded for provenance): form snapshot tests (filter_drawer_overlay etc.) stay CENTRAL, to be organized in the render phase -- not migrated per-module. tui.rs 6862->6628. cargo test -p yaks 242+25 PASS, 22 snaps intact. Remaining: create/edit form -> src/tui/create.rs.

---
▸ 2026-09-11T03:04:36Z [coordinator]
verify: `cargo test -p yaks` -> PASS (exit 0)

---
▸ 2026-09-11T03:04:36Z [coordinator]
Shorn (create half done): moved ContentBlock + CreateForm + impl CreateForm + render_create/render_content_stack/render_block_separator into src/tui/create.rs (pub(crate)). kind_index/pri_index stay in tui.rs (App uses them) reached via super::; shared render helpers (render_chip_row/render_text_row) + field helpers + consts stay central. Removed now-unused Wrap import from tui.rs. Form snapshot tests stay central per the phase-2 convention. tui.rs 6862->6319 (7043 original). cargo test -p yaks 242+25 PASS, 22 snaps intact, no warnings.
