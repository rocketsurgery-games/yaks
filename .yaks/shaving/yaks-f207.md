---
id: yaks-f207
title: Changing priority (or anything that causes a list resort) in detail mode doesn't update selection
type: bug
priority: 3
created: '2026-08-23T18:26:53Z'
updated: '2026-09-10T23:47:07Z'
labels:
- ui
needs: human
verify: cargo test -p yaks
---

Presumably the selection model is index-based, so when you change a property that updates the current yak's sort order, the rendering updates to point at whatever yak is *now* in its slot. Would be nice if it actually followed the new sort position (if it's still in the current list). If it drops *out* of the list, we should probably just close the detail panel (or leave it up, unchanged, and no longer tracking a list position; if that's a thing the UI structure can represent).

---
▸ 2026-09-10T23:47:07Z [coordinator]
LANE C (wt/resort). Scope: src/tui.rs apply_edit (3149) ONLY -- one-liner: self.reload() -> self.reload_preserving_selection() (already exists at 975); NO new App field. Behavior: after a resort-causing edit the cursor follows the yak to its new sorted slot. Drop-out-of-list behavior is BLOCKED on a human decision (see the ask below). Evidence contract: insta snapshot proving selection follows a priority change; append new test near the selection tests (~5840). JUDGE = script (cargo test -p yaks) -> self-shear on green. HELD until f207 is answered.

---
▸ 2026-09-10T23:47:07Z [coordinator]
Design decision needed before lane C starts: when a priority change pushes the selected yak OUT of the current filtered/sorted list entirely, what should the detail pane do? (a) close the detail pane; or (b) keep it open, unchanged, no longer tracking a list slot. Coordinator lean: (a) close -- simplest, matches 'the thing you were looking at is no longer here'. Please 'yaks answer yaks-f207 --note ...' with a or b.
