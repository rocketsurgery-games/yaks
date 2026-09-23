---
id: yaks-b0b4
title: Edit source frontmatter in the create/edit form
type: bug
priority: 2
created: '2026-08-23T02:49:48Z'
updated: '2026-09-23T21:42:56Z'
labels:
- ui
---

You have to edit the yak file manually to add or update this field. Super annoying.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'form' (worktree wt/form, branch lane-form). Scope: add a Source field to the create/edit form (create.rs + form render + farm write path). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![lane-form-source-field](artifacts/yaks-b0b4/lane-form-source-field.txt)

---
▸ 2026-09-23T21:42:48Z [lane-form]
Headless frame (72x14): create form with the new 'source' row focused and a URL typed. Snapshot: create_form_source_field.

![tui-edit-herd](artifacts/yaks-b0b4/tui-edit-herd.svg)

---
▸ 2026-09-23T21:42:48Z [lane-form]
Regenerated docshot: edit form (multi-herd) now shows the source row between labels and herd.

---
▸ 2026-09-23T21:42:56Z [lane-form]
Shorn: create/edit form gains a single-line 'source' row (row 4, under labels; HEADER_ROWS 4->5, herd row moves below it). Create sets NewTask.source (empty = none); edit diffs against the current source and sends TaskEdit.source. Farm::update now treats source "" as clear (like verify) — so 'yaks update --source ""' clears too; clap help + docs/cli.md + skills/yaks updated. E on the detail 'Source:' line focuses the row (EditTarget::Source). Tests: create_form_source_field (snapshot), live::create_form_source_is_saved, live::edit_form_source_set_and_clear, e_on_source_line_opens_form_on_the_source_row; existing tab-count tests adjusted; create_form/edit_form_panel snapshots + tui-edit-herd.svg docshot re-accepted (one extra 'source' row). Gate green.
