---
id: yaks-96b6
title: b1cc/3 Extract drawer + create/edit form -> src/tui/{drawer,create}.rs
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-11T01:30:07Z'
parent: yaks-b1cc
depends_on:
- yaks-7483
labels:
- ui
verify: cargo test -p yaks
---

drawer.rs: Drawer + consts + text_field/multiline_field/toggle + impl Drawer + render_drawer/render_chip_row. create.rs: ContentBlock/CreateForm + kind_index/pri_index + impl CreateForm + render_create/render_content_stack/render_block_separator/render_text_row. Move tests + snapshots. Green.
