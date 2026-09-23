---
id: yaks-9e59
title: Max detail with affordance
type: feature
priority: 3
created: '2026-08-28T04:00:19Z'
updated: '2026-09-23T21:40:37Z'
labels:
- ui
---

It would be nice for the detail pane width to grow to a _point_, but then stop growing, so it doesn't get too wide, and leaves more room for the list/tree view.
This could be configurable, but as a starting point I think it would be fine to start with our existing %, with a simple min/max detail width. Give the list view whatever's leftover, so that eventually the detail view just covers the whole thing, making it effectively modal.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'width' (worktree wt/width, branch lane-width). Scope: detail pane min/max width layout. Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![lane-width-frames](artifacts/yaks-9e59/lane-width-frames.txt)

---
▸ 2026-09-23T21:40:17Z [lane-width]
Headless frames at 60 (modal, ◂ affordance), 100 (unclamped 34/66), 220 (detail clamped to 100 cols; list gets 120)

---
▸ 2026-09-23T21:40:37Z [lane-width]
Shorn: detail/right-pane width = 66% clamped to [48,100] cols via render::main_split (the single split helper; mouse lane can reuse it). Consts DETAIL_PCT/DETAIL_MIN_WIDTH/DETAIL_MAX_WIDTH/LIST_MIN_WIDTH. If the list would get <24 cols, the pane covers the main area (modal) and a cyan ◂ tips its divider; the help bar already shows h:list. No snapshot ripple because the unclamped range matches the old 34/66 split column for column. Tests: main_split at 40/71/72/80/140/220 plus a frame test for the modal affordance. cargo test -p yaks green (282 unit + 25 cli). docs/tui.md updated.
