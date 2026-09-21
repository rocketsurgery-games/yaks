---
id: yaks-71d1
title: Edit affordance for yak herd
type: feature
priority: 3
created: '2026-09-21T20:32:04Z'
updated: '2026-09-21T23:33:19Z'
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

---
▸ 2026-09-21T21:17:07Z [Joel Webber]
We're still missing a 'herd' line in the edit UI, and the TUI help doesn't show this key anywhere that I can discern.
When we fix this, we should also follow up with some AGENTS.md guidance on ensuring that all the docs / help / --help are updated.

---
▸ 2026-09-21T23:33:13Z [agent]
Reopen follow-up: the H herd-move was List-only and undiscoverable. Added (1) a 'Herd:' field line in the detail/edit pane (detail::build), shown between Type and Priority on multi-herd farms only (mirrors the id-colour present-prefix gate); (2) H to the Focus::Detail key arm so herd is editable from the full edit UI; (3) an H entry in the ? help overlay (help_content). Docs: docs/tui.md Signals note. New tests: detail_tests::herd_line_shows_only_on_a_multi_herd_farm, live::herd_pick_works_from_the_detail_pane. Smoked headless on this repo: detail shows 'Herd: yaks', H opens the picker in detail, and ? lists the key. Also expanded AGENTS.md 'Keeping docs current' to enumerate --help + the ? overlay/help bar as required surfaces (the gap that let this ship half-done).

---
▸ 2026-09-21T23:33:19Z [agent]
verify: `cargo test --workspace` -> PASS (exit 0)
