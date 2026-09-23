---
id: yaks-7cb3
title: Parse label edits with/without commas
type: bug
priority: 3
created: '2026-08-26T13:23:02Z'
updated: '2026-09-23T21:35:37Z'
parent: yaks-f04d
labels:
- ui
- labels
---

At present, it's possible to create labels with spaces in them. We should disallow this.
Then we can enforce label structure at parse time (we should probably normalize them in the CLI tools as well).
If the user types:
- `foo, bar` -> `[foo bar]`
- `foo bar`  -> `[foo bar]`
- `foo,bar`  -> `[foo bar]`

And so forth. Ie, we should disallow spaces and commas in labels.

---
▸ 2026-09-10T23:46:27Z [coordinator]
CLI evidence (UI-batch recon): 'yaks create --labels meta,skills' stores a SINGLE joined label 'meta,skills' -- --labels is space-variadic and does not split on comma. Existing herd yaks show the same smell (ui,docs; skills,eval; ui,editing). Whatever normalization this yak lands for the TUI label editor should also cover the CLI --labels/--add-label path: split+trim on comma/space, drop empties.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'labels' (worktree wt/labels, branch lane-labels). Scope: label normalization (split on comma/space, no spaces/commas) in store/CLI/TUI label editor. Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.
