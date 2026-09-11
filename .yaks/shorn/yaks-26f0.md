---
id: yaks-26f0
title: Search in detail not sticky
type: bug
priority: 3
created: '2026-08-28T05:02:53Z'
updated: '2026-09-10T23:55:03Z'
labels:
- ui
verify: cargo test -p yaks
---

When searching for text within a yak detail, it correctly cycles through all the results with n/N. But when you try to Enter out of this mode, the cursor jumps back to wherever it was before the search. I think it's ok for Esc to drop you back to the old location like vi does. But "enter" after selecting one should leave the cursor on the match.

---
▸ 2026-09-10T23:47:07Z [coordinator]
LANE B (wt/detailfind). Scope: src/tui.rs Overlay::DetailFind branch (2744-2771) + open_detail_find (2347) + the DetailFind/SearchBox overlay struct (stash pre-find scroll+line) ONLY; reuse existing detail_* fields (NO new App field). Behavior: Enter leaves the detail cursor (detail_line) ON the current match (and scrolled there); Esc restores the pre-find scroll+line (vi-like). Root cause: Enter branch at 2753-2754 closes the overlay without moving detail_line. Evidence contract: insta snapshot(s) proving cursor-on-match after Enter + restore-on-Esc; append new tests near the existing detail_find_* tests (~6100). JUDGE = script (cargo test -p yaks) -> self-shear on green.

---
▸ 2026-09-10T23:54:55Z [Joel Webber]
verify: `cargo test -p yaks` -> PASS (exit 0)

---
▸ 2026-09-10T23:55:01Z [lane-b]
Fixed detail-find Enter/Esc (src/tui.rs): Enter now sets detail_line to detail_find_matches()[detail_match].0 (cursor lands ON the current match, scroll preserved); Esc/Ctrl-C restores the pre-find (detail_scroll, detail_line) stashed in SearchBox.detail_origin at open_detail_find. Evidence (all PASS): new tests detail_find_enter_lands_cursor_on_match, detail_find_enter_commits_the_cycled_match, detail_find_esc_restores_pre_find_position. cargo test -p yaks: 239 unit + 25 cli PASS. yaks verify yaks-26f0: PASS (exit 0). Docs: docs/tui.md detail-find line updated with Enter/Esc semantics.
