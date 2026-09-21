---
id: yaks-a7eb
title: TUI herd badge (per-herd id colour)
type: feature
priority: 3
created: '2026-09-21T19:19:35Z'
updated: '2026-09-21T19:31:02Z'
parent: yaks-15f7
labels:
- ui
---

Polish deferred from yaks-3290. (1) Per-row herd badge/column in the list, gated to multi-herd farms (>1 distinct prefix) so single-herd is unchanged. NOTE: the id already shows the herd via its prefix, so weigh a distinct colored column vs redundancy before building. (2) Explicit in-form herd picker in the create form (a chip row like type/priority) so the target herd can be chosen, not only inherited from context; needs create-form row-index surgery (HEADER_ROWS, is_line_text_row, line_editor, move_chip, render layout), done carefully behind the multi-herd gate. Today create seeds the target herd from the selected row's/parent's prefix and shows it in the header (yaks-3290); CLI --prefix gives explicit control.

---
▸ 2026-09-21T19:30:58Z [agent]
SHORN (badge). Per-herd colour on each row's id prefix in the list, gated to multi-herd farms (App::is_multi_herd = more than one distinct id prefix), so single-herd farms and snapshots are unchanged. render::herd_color hashes the prefix (FNV-1a) into a 6-colour palette; the id tail stays blue. Colours do not appear in the plain-text snapshots (no churn); verified by unit tests (is_multi_herd plus a multi-herd draw). The explicit in-form herd PICKER (override the target herd inside the create form) is split into a standalone follow-up: it needs create-form row-index surgery, and the context-seeded current-herd plus CLI --prefix already cover routing, so it is kept out of the release. docs/tui.md notes the herd colour and create-header herd.
