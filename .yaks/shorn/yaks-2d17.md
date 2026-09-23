---
id: yaks-2d17
title: Broken coloring in wrapping yak detail lines
type: bug
priority: 3
created: '2026-09-21T23:49:17Z'
updated: '2026-09-23T21:43:06Z'
labels:
- ui
---

Header lines that wrap in the detail view get the bright white coloring reserved for field names, in the bits where they fall on the same columns. See attached image.

![paste-20260921-234927](artifacts/yaks-2d17/paste-20260921-234927.png)

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'polish' (worktree wt/polish, branch lane-polish). Scope: detail header line wrapping/coloring (detail.rs/render.rs). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![yaks-2d17-before](artifacts/yaks-2d17/yaks-2d17-before.svg)

---
▸ 2026-09-23T21:42:50Z [lane-polish]
BEFORE (fix reverted): wrapped Title continuation rows take the dim label colour in cols 0-12 ('role Choice (', 'standalone St').

![yaks-2d17-after](artifacts/yaks-2d17/yaks-2d17-after.svg)

---
▸ 2026-09-23T21:42:50Z [lane-polish]
AFTER: continuation rows are uniformly value-coloured; only the 'Title:' label on the first row is dim.

![2d17-after](artifacts/yaks-2d17/2d17-after.png)

---
▸ 2026-09-23T21:42:50Z [lane-polish]
Chrome rasterization of the AFTER svg.

---
▸ 2026-09-23T21:43:06Z [lane-polish]
Shorn. Cause: render_dline (src/tui/render.rs) dims the first 13 chars of any link-less Kind::Field line as the label; detail::wrap marks soft-wrapped continuation rows cont=true but keeps kind=Field, so each continuation row got the label colour in cols 0-12. Fix: label_end requires !dl.cont. Regression test (TestBackend cell styles): tui::tests::wrapped_detail_field_continuation_is_not_label_coloured — fails with the fix reverted (cell 36 took the label colour), passes with it. Before/after SVGs attached (before reproduces the screenshot). cargo test -p yaks --bins green (284), toque green, tests/cli green after build.
