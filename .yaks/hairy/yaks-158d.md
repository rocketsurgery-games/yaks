---
id: yaks-158d
title: Global back/forward navigation across view states
type: feature
priority: 3
created: '2026-08-23T02:49:48Z'
updated: '2026-09-23T21:39:42Z'
labels:
- ui
- search
needs: human
---

Generalize the (shorn) detail-scoped nav stack from yak-2d13 into a global back/forward history spanning ALL view/UI states, not just yak-detail drill-down. Today nav_history/nav_pos is reset on every _enter_detail and _detail_next_task, so it only tracks navigation within a single detail context.

Goal: one navigation stack recording view switches AND task navigations, so back/forward returns you to the exact prior UI state (which view, cursor, filter). This is the true 'recently viewed' affordance deferred from yak-597c: it covers the 'looked at but did not change' case that the Recent view (derived from updated:) deliberately does not.

Many moving parts: what counts as a nav event; how filter/ephemeral-view state is snapshotted and restored; persistence across sessions; interaction with the 500ms auto-reload. Break into research + design before implementing.

Adjacent: yak-6f33 (carry search context globally) and yak-2d13 (shorn predecessor: detail-scoped nav stack).

---
▸ 2026-09-23T16:57:17Z [Joel Webber]
Bonus points: I find it annoying when I i/o my way in and out of a stack, and lose my list context in the process.
It's definitely helpful that moving up and down the stack makes a point to find the yak in *some* list, but it would be even better if that list context were part of the actual stack, so it brings me back reliably to where I was previously.

![158d-design](artifacts/yaks-158d/158d-design.md)

---
▸ 2026-09-23T21:39:42Z [design-scout]
Design proposal attached

---
▸ 2026-09-23T21:39:42Z [design-scout]
Design attached (158d-design.md). Root cause: every jump goes through select_task -> set_view(status view), which resets filter + cursor; entries only hold ids. Proposal: NavEntry = {view_key, live filter, cursor id/idx, list offset, focus, 28b4 detail pos}, snapshot on departure. Decisions:
1) What pushes? Lean: jumps only (link follow, view switch, programmatic goto). Filter/search commits and entering/leaving detail update the current entry in place and don't push.
2) Link follow to a yak outside the current list: lean: stay in the current list if the target is in it, else switch to its status view as today (now undoable with o). Also fixes a bug: following a link to a dead yak shows the previous yak.
3) Keys: lean: o/i in list focus too (both free), and no Ctrl-O/Ctrl-I primaries (Ctrl-I = Tab without kitty).
4) Persist across sessions? Lean: no for the stack; maybe later persist just the last position so the TUI reopens where you quit.
