---
id: yaksrs-14bc
title: Missing [c]hange actions
type: feature
priority: 3
created: '2026-08-26T13:34:07Z'
updated: '2026-08-27T16:37:39Z'
parent: yaksrs-fc85
labels:
- ui
- editing
---

They should work just like they do in vi.

---
▸ 2026-08-27T16:37:39Z
Implemented on the edtui fork. The fork already had cw/cW/cf/ct/ciw/ciW/ci<pair> and visual change; the everyday operators that were missing are now added: C (c$, change to end of line), cc / S (change whole line -- clear content, keep the line), and s (cl, substitute char under cursor, count-aware). Each delegates to the matching delete primitive then enters Insert, mirroring the existing c-operators. Authored on branch yaks/change-actions off main (for upstream), merged into yaks-integration (added Substitute::set_count there since main's Execute predates counts). Fork lib: 159 tests green. yaks pin bumped (Cargo.lock -> 9fbee79); yaks-side tests confirm cc + s route through. 154 bin green, 0 warnings.
