---
id: yaks-fe19
title: Rename attachment
type: feature
priority: 3
created: '2026-09-22T00:10:18Z'
updated: '2026-09-23T21:35:37Z'
labels:
- cli
- ui
---

Pasted PNGs tend to have ugly names. It would be nice to be able to rename them in the UI. And, for that matter, from the CLI as an option to `yaks --attach`.
Bonus points for an attach-time affordance in the "empty to paste PNG from clipboard" flow.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'attach' (worktree wt/attach, branch lane-attach). Scope: rename attachments: CLI + TUI (a09b is the TUI half). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.
