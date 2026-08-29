---
id: yaks-3b22
title: 'Parity: E opens a full edit form (share the create-form widget)'
type: task
priority: 3
created: '2026-08-22T16:03:41Z'
updated: '2026-08-22T19:42:02Z'
parent: yaks-0a93
labels:
- rust
- tui
- parity
---

Python's E reopens the whole task form (title/type/priority/labels/description) in edit mode; Rust's E edits only the description body (other fields via L/P/T/S). Now that the create form exists (d13b: Overlay::Create/CreateForm), refactor it into a shared create/edit widget seeded from an existing task, and route E through it. Aligns with the goal of a single shared editor implementation. See docs/tui-parity.md #8 and #12. Surfaced by a083.

---
▸ 2026-08-22T19:42:02Z
Generalized CreateForm into a shared create/edit form. Added edit_id + CreateForm::for_edit (seeds title/type/priority/labels/description from a task); open_edit routes E (list + detail) to it. Description is now a multi-line edtui content zone (multiline_field) with a '─ description ─' separator, Enter=newline. Commit gesture switched to Ctrl-S (create -> Herd::create; edit -> Herd::update, field-diffed so unchanged fields don't rewrite/bump updated); Esc/Ctrl-C cancel (Esc yields to the editor inside the description zone). Removed the old body-only editor (open_body_edit + EditAction::Body). Reusable content zone for future comment editing. Tests updated to Ctrl-S; added edit_form_updates_description/changes_type_and_priority/cancel + edit_form_panel snapshot; create_form snapshot regenerated. 108 unit + 19 CLI green, warning-free, no new clippy. Verified multi-line body persists end-to-end.
