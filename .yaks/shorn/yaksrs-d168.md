---
id: yaksrs-d168
title: 'edtui PR: count prefixes (3j, 2dd, 5x)'
type: feature
priority: 2
created: '2026-08-24T22:12:10Z'
updated: '2026-08-25T03:52:21Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
source: https://github.com/preiter93/edtui/pull/72
---

Marquee contribution. KeyEventHandler has no numeric-count concept, but nearly every Action already carries an unused usize count (MoveDown/DeleteLine/RemoveChar/DeleteWordForward...). Route a numeric register accumulated in the key handler into those counts. High value; codebase is pre-shaped for it. Requires GitHub fork.

---
▸ 2026-08-25T02:37:46Z
Implemented + opened PR preiter93/edtui#72 from branch yaks/count-prefixes. Approach: count register on KeyEventHandler accumulates leading digits (normal/visual); applied via a new Execute::set_count (default no-op) overridden on the 21 count-bearing actions, so counts drive the actions' EXISTING usize fields -> undo/register stay correct (2dd = one undo, yanks both lines). Bare leading 0 stays a motion; mid-sequence digits fall through. Covers 3w/5x/2dd/3dw/3j etc. Not handled (noted in PR): d3w (operator+count+motion), 3fx (counted char-arg), count-aware dot-repeat. Added test_count_prefix; cargo test --lib green (150). Built with 1.95 toolchain.

---
▸ 2026-08-25T03:52:21Z
Upstream PR withdrawn/closed (opened prematurely). Implementation is preserved on its fork branch for the user to review; re-upstreaming will go issues-first. Kept shorn since the code work is complete.
