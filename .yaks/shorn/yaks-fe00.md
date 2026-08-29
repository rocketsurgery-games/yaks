---
id: yaks-fe00
title: 'Dead status is a dead-end in the TUI: drawer offers it but views load include_dead=false'
type: bug
priority: 3
created: '2026-08-28T03:55:07Z'
updated: '2026-08-28T04:28:30Z'
labels:
- ui
---

The filter drawer STATUS_CHOICES includes Dead, but App.all is always loaded via herd.list(.., include_dead=false) (with_herd plus both reload paths), so filtering to Dead shows an empty pane and a dead ancestor silently breaks a child parent-chain. Decide: make Dead reachable via an explicit dead-aware load path, or drop Dead from the drawer and make it view-only. Related: the yaks-78dc herd-scope design wants the ancestor walk to see Dead even when the anchor scope excludes it, so the parent chain never breaks. Split out of yaks-78dc.

---
▸ 2026-08-28T04:28:30Z
Implemented (Option B: make Dead reachable). App.all now loads every status (herd.list include_dead=true) in with_herd + both reload paths, so dead lives in the model. Effects: a dep on a slaughtered yak now resolves (previously it falsely showed blocked); the ancestor walk can root a live yak beneath a dead parent; selecting Dead in the filter drawer surfaces slaughtered yaks. Flat views (Recent/custom) guard against leaking dead unless the live filter requests Dead; tree anchors already exclude Dead by default and remaining prunes dead-only subtrees. Tests: slaughter_confirm_moves_to_dead updated (dead persists in the model but is hidden), plus new dead_is_loaded_but_hidden_until_filtered.
