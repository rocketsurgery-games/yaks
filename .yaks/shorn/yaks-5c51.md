---
id: yaks-5c51
title: Apply the slaughter child-guard to bulk/multi-select slaughter
type: bug
priority: 2
created: '2026-09-05T03:18:30Z'
updated: '2026-09-05T03:23:19Z'
parent: yaks-2ebe
labels:
- ui
---

Safety follow-up flagged in yaks-de85: single-yak slaughter (open_slaughter_confirm) refuses a yak with children ('slaughter them first'), but the multi-select bulk path (PickAction::BulkState 'x') loops herd.transition to Dead WITHOUT that guard. Apply the same per-child guard to the bulk slaughter path (skip + report ids that have children).

---
▸ 2026-09-05T03:23:19Z [wt-tui]
Fixed bulk slaughter (PickAction::BulkState 'x'): now skips any marked id with live children and reports 'N skipped: have children', mirroring open_slaughter_confirm. Extracted shared App::live_child_count helper (single path reuses it). Non-slaughter bulk transitions (h/s/n) unchanged. Regression test tui::tests::live::bulk_slaughter_skips_yaks_with_children: marks parent-with-child + childless yak, bulk-slaughters, asserts parent+child survive (Hairy) + childless is Dead, notification '1/2 -> dead . 1 skipped: have children'. OPTIONAL done (clean): mirrored 'm' mark into detail pane + test detail_pane_mirrors_multi_select_mark. cargo test --workspace --release: 232 yaks + 13 toque + 24 cli + 1 doc, 0 failed. Scope: src/tui.rs only.
