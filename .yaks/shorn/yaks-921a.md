---
id: yaks-921a
title: Default to normal vi mode in desc/comment editing
type: task
priority: 3
created: '2026-08-26T12:49:12Z'
updated: '2026-08-26T13:50:45Z'
parent: yaks-fc85
depends_on:
- yaks-a031
labels:
- ui
---

That's by far the most common expectation for vi users.

---
▸ 2026-08-26T13:50:45Z
Implemented as a content-based heuristic rather than unconditional Normal: in vim, a multiline desc/comment editor seeded with existing content opens in Normal mode (the vi expectation when reviewing text), while an empty one opens in Insert so you can type immediately. So editing an existing yak's description or a comment (E) lands in Normal; a new yak's description (c/C) and a new comment (M) land in Insert. Emacs and single-line fields unchanged (always Insert). Implemented in multiline_field + Editor::new. Tests: seeded->Normal, new->Insert. The edit_form_panel snapshot was unaffected (text snapshots capture characters, not the cursor-cell style that distinguishes the modes). 150 bin green, 0 warnings.
