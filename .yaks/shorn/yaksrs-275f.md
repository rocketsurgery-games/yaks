---
id: yaksrs-275f
title: Add confirm-on-cancel step for dirty yaks
type: feature
priority: 3
created: '2026-08-26T11:59:40Z'
updated: '2026-08-26T13:55:32Z'
parent: yaksrs-fc85
labels:
- ui
---

Whether by Ctrl-C or double-Esc. Only if a field has changed.

---
▸ 2026-08-26T13:02:05Z
Overlaps with yaksrs-683f (save/preserve incomplete edits as a draft on cancel). Same 'don't lose work on cancel' problem, different answers -- design them together; draft-preservation may reduce the need for a confirm prompt.

---
▸ 2026-08-26T13:55:32Z
Implemented. Cancelling a *dirty* edit surface now pops a 'Discard changes? (y/N)' confirm instead of silently dropping work; declining restores the stashed editor intact. App.request_cancel stashes the overlay in App.dirty_cancel and shows a Confirm(DiscardEdit); y -> discard (notification 'changes discarded'), n/Esc/Enter -> restore. Dirtiness: create/edit form via form_is_dirty (vs the seeded task when editing, vs empty defaults when creating); the multiline comment editor is dirty when non-empty. Scope: create/edit form + comment editor only -- single-line field edits (labels/rename/save-view/attach) and clean cancels skip the prompt. Complements yaksrs-683f (draft preservation is an orthogonal approach to the same concern). Tests: dirty create/edit/comment -> confirm -> discard; decline restores the form with work intact; clean cancel is immediate. 152 bin green, 0 warnings.
