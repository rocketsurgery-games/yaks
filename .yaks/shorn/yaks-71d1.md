---
id: yaks-71d1
title: Edit affordance for yak herd
type: feature
priority: 3
created: '2026-09-21T20:32:04Z'
updated: '2026-09-21T21:02:22Z'
labels:
- herds
- tui
---

Otherwise you're forced to fall back to `yaks rename old-... new-...`. Ain't nobody want that.

---
▸ 2026-09-21T20:59:27Z [agent]
Shipped: TUI 'H' key in the List pane opens a single-key herd picker (PickAction::Herd) offering the other herds; picking one renames <old>-<tail> to <new>-<tail> via farm.rename, handling all RenameOutcome variants. Single-herd farms no-op with a hint. Gated on is_multi_herd(). Two live tests in tui/tests.rs; docs/tui.md key row added. Smoked headless on this repo's yaks+test farm: H on yaks-9009 offers 1=test.

---
▸ 2026-09-21T21:02:14Z [agent]
verify: `cargo test --workspace` -> PASS (exit 0)
