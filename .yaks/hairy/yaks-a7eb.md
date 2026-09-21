---
id: yaks-a7eb
title: TUI herd badge + explicit in-form herd picker
type: feature
priority: 3
created: '2026-09-21T19:19:35Z'
updated: '2026-09-21T19:19:35Z'
parent: yaks-15f7
labels:
- ui
---

Polish deferred from yaks-3290. (1) Per-row herd badge/column in the list, gated to multi-herd farms (>1 distinct prefix) so single-herd is unchanged. NOTE: the id already shows the herd via its prefix, so weigh a distinct colored column vs redundancy before building. (2) Explicit in-form herd picker in the create form (a chip row like type/priority) so the target herd can be chosen, not only inherited from context; needs create-form row-index surgery (HEADER_ROWS, is_line_text_row, line_editor, move_chip, render layout), done carefully behind the multi-herd gate. Today create seeds the target herd from the selected row's/parent's prefix and shows it in the header (yaks-3290); CLI --prefix gives explicit control.
