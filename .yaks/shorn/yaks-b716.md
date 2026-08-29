---
id: yaks-b716
title: Normal-mode j/k navigates between fields in single-line editors
type: feature
priority: 3
created: '2026-08-28T04:51:36Z'
updated: '2026-08-28T04:58:16Z'
labels:
- ui,editing
---

Follow-up polish to yaks-efa9. In a single-line field in vim Normal mode, j/k do nothing (one line, no vertical motion). Repurpose them to move to the prev/next field/row, exactly like Tab / Ctrl-N/P already do for non-editor (chip) rows -- so form and drawer navigation feels vi-native. Applies to: create/edit form single-line rows (title/labels), filter drawer text rows (labels/search/parent), and the fuzzy picker query (j/k move the result selection). Insert mode still types j/k. Multiline content blocks keep j/k as line motion.

---
▸ 2026-08-28T04:58:16Z
Implemented. In vim Normal mode, j/k now navigate between fields instead of being no-ops: create/edit form single-line rows (title/labels), filter drawer text rows (labels/search/parent), and the fuzzy picker (j/k move the result selection). A j/k field-nav carries Normal to the destination field so it keeps moving (esp. across the drawers adjacent text rows) rather than dropping back to Insert; Insert-mode j/k still type. Added CreateForm::line_editor and Drawer::text_editor helpers. Tests: drawer_normal_jk_navigates_and_carries_mode, create_form_normal_j_navigates_rows, fuzzy_normal_jk_moves_selection.
