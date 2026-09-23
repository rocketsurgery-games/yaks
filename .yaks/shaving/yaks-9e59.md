---
id: yaks-9e59
title: Max detail with affordance
type: feature
priority: 3
created: '2026-08-28T04:00:19Z'
updated: '2026-09-23T21:35:37Z'
labels:
- ui
---

It would be nice for the detail pane width to grow to a _point_, but then stop growing, so it doesn't get too wide, and leaves more room for the list/tree view.
This could be configurable, but as a starting point I think it would be fine to start with our existing %, with a simple min/max detail width. Give the list view whatever's leftover, so that eventually the detail view just covers the whole thing, making it effectively modal.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'width' (worktree wt/width, branch lane-width). Scope: detail pane min/max width layout. Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.
