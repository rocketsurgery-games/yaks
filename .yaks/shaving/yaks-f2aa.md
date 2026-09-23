---
id: yaks-f2aa
title: Pasting into description *very* slow
type: bug
priority: 1
created: '2026-08-30T19:05:01Z'
updated: '2026-09-23T21:35:37Z'
labels:
- ui
---

It works fine with the native paste affordance, but when you ctrl/cmd-v, it's excruciatingly slow, echoing out characters like a 300bps modem!

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'paste' (worktree wt/paste, branch lane-paste). Scope: Cmd/Ctrl-V paste performance in editors (runtime event loop / bracketed paste / editor glue). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.
