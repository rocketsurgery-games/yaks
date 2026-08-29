---
id: yaks-20b9
title: Extract headless harness into the toque crate (workspace member)
type: task
priority: 2
created: '2026-08-22T21:58:56Z'
updated: '2026-08-22T22:19:35Z'
labels:
- rust
- tui
- crate
---

Split src/tui/headless.rs into a publishable workspace member 'toque' (chef's-hat pun: drive a ratatui app from under the toque). Root stays the yaks package + workspace root; add crates/toque. Public API: HeadlessApp trait (render/handle_key/on_resize/state_header/should_quit), StyleEncoding, SnapshotEncoder (owns the stable-id StyleRegistry), render_to_buffer, run (stdin/stdout driver), parse_key. on_resize replaces the driver mutating app.page/detail_page (kills the paging leak). yaks becomes the first consumer. README distilled from docs/tui-style-eval.md (encodings considered + rough results).

---
▸ 2026-08-22T22:19:35Z
Done. crates/toque extracted: HeadlessApp trait (render/handle_key/on_resize/state_header/should_quit), StyleEncoding, SnapshotEncoder (stable-id registry), render_to_buffer, Session, run, parse_key. on_resize kills the driver's page/detail_page mutation (yaks derives paging in its impl). Root is now a workspace (root yaks pkg + crates/toque); yaks is first consumer via src/tui/headless.rs adapter. README distilled from tui-style-eval.md. toque: 13 unit + 1 doctest, clippy-clean; yaks: 109 unit + 19 CLI. Warning-free. NOTE: test cmd is now 'cargo test --workspace'.
