---
id: yaks-b0b4
title: Edit source frontmatter in the create/edit form
type: bug
priority: 2
created: '2026-08-23T02:49:48Z'
updated: '2026-09-23T21:35:37Z'
labels:
- ui
---

You have to edit the yak file manually to add or update this field. Super annoying.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'form' (worktree wt/form, branch lane-form). Scope: add a Source field to the create/edit form (create.rs + form render + farm write path). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.
