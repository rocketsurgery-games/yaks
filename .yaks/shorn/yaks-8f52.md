---
id: yaks-8f52
title: 'TUI: explicit in-form herd picker (override the create target)'
type: feature
priority: 3
created: '2026-09-21T19:30:58Z'
updated: '2026-09-21T19:42:04Z'
labels:
- ui
---

Follow-up split from yaks-a7eb. Today the TUI create form seeds its target herd from context (parent or selected row prefix) and shows it in the header (yaks-3290); CLI --prefix gives explicit control. Add an explicit picker to OVERRIDE the seeded herd inside the create form in a multi-herd farm (e.g. create a web yak while sitting on an api row). Cleanest UX is a chip row like type and priority, gated to multi-herd farms, which needs create-form row-index surgery: HEADER_ROWS becomes a form method, plus is_line_text_row, line_editor, move_chip, content_index, row_count, render_create layout, and open_edit_focus. Deliberately deferred from the family release to avoid touching the core create/edit form under time pressure; the family core goal is met without it.

---
▸ 2026-09-21T19:42:04Z [agent]
SHORN. Explicit in-form herd PICKER in the create form. CreateForm carries herds: Vec<String> + herd_idx (replacing the context-only Option); render_create adds a 'herd' chip row (like type/priority) AFTER labels (row index HEADER_ROWS) so rows 0-3 are untouched; header_rows/content_index/row_count/move_chip account for it. The row and the id colouring are gated to multi-herd farms (App::is_multi_herd via herd_choices = declared config herds unioned with prefixes present). open_create seeds the selection to the reference yak's herd (parent/selected row); commit passes target_herd() as the new yak's prefix. Single-herd farms unchanged (no row, no snapshot churn). Tests: picker present+cycles+renders, existing create-routing moved to target_herd(); suite green (257 lib + 25 cli), warning-free. Verified in the real TUI on THIS repo (a demo 'test' herd added to .yaks/config.yaml): create form shows a 'herd  test yaks' chip row; verify test-9d7a resolved the test herd's lever; list --herd test scoped.
