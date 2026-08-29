---
id: yaks-127e
title: Structured description / comment navigation and editing
type: feature
priority: 2
created: '2026-08-25T02:44:04Z'
updated: '2026-08-26T13:35:10Z'
labels:
- ui
---

Currently the "description", whether viewing or editing, encompasses both the description and all the comments.

It would be more effective if one could navigate among the description and comments like other fields, at least when editing.

The viewing case is a little less clear -- line selection is important and useful, as is tabbing among the links. One idea --
use ctrl-N/P (just like it does in editing) to move among the blocks, in *both* modes. I think we could make that make sense,
without being in conflict with line-selection and tabbing in view mode.

---
▸ 2026-08-26T12:17:13Z
Implemented. New src/tui/content.rs: round-trip-safe body<->[Description, Comment*] parser (splits only on the exact ---/marker shape; a bare --- rule stays in the description; assemble drops empty comments = delete-on-empty). Form refactor: CreateForm now holds blocks: Vec<ContentBlock> (description + one per comment); Ctrl-N/P/Tab walk every row and the focused block expands to a live editor (accordion), unfocused blocks collapse to labeled separators (description shows a dimmed preview when a header field is focused). Save reassembles the body via content::assemble. E is now context-sensitive from the detail pane (open_edit_at_cursor -> edit_target_at maps the line cursor to Title/Type/Priority/Labels/Status or the specific content block; Status routes to its picker). View-mode Ctrl-N/P (jump_block) moves the line cursor among block starts via detail::block_index_per_line. Timestamps preserved on edit. Tests: 7 content parser + 3 view-side (E-on-comment/E-on-title/Ctrl-N-P nav) + 3 live commit (edit-in-place, no-op round-trip, delete-on-empty). Full workspace green (145 bin), 0 warnings.
