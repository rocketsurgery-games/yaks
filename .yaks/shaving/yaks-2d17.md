---
id: yaks-2d17
title: Broken coloring in wrapping yak detail lines
type: bug
priority: 3
created: '2026-09-21T23:49:17Z'
updated: '2026-09-23T21:35:37Z'
labels:
- ui
---

Header lines that wrap in the detail view get the bright white coloring reserved for field names, in the bits where they fall on the same columns. See attached image.

![paste-20260921-234927](artifacts/yaks-2d17/paste-20260921-234927.png)

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'polish' (worktree wt/polish, branch lane-polish). Scope: detail header line wrapping/coloring (detail.rs/render.rs). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.
