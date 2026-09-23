---
id: yaks-28b4
title: Keep detail cursor/scroll position on back/forward nav
type: bug
priority: 3
created: '2026-09-07T02:38:01Z'
updated: '2026-09-23T21:35:38Z'
labels:
- ui
---

When navigating forward and backward through yak detail links, we lose the cursor/scroll position. We should preserve this as part of the stack.
Relates to yaks-158d; consider implementing together.

---
▸ 2026-09-23T21:35:38Z [coordinator]
Lightning-round lane 'navpos' (worktree wt/navpos, branch lane-navpos). Scope: preserve detail cursor/scroll in the detail nav stack (detail_nav.rs). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.
