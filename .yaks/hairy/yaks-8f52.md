---
id: yaks-8f52
title: 'TUI: explicit in-form herd picker (override the create target)'
type: feature
priority: 3
created: '2026-09-21T19:30:58Z'
updated: '2026-09-21T19:30:58Z'
labels:
- ui
---

Follow-up split from yaks-a7eb. Today the TUI create form seeds its target herd from context (parent or selected row prefix) and shows it in the header (yaks-3290); CLI --prefix gives explicit control. Add an explicit picker to OVERRIDE the seeded herd inside the create form in a multi-herd farm (e.g. create a web yak while sitting on an api row). Cleanest UX is a chip row like type and priority, gated to multi-herd farms, which needs create-form row-index surgery: HEADER_ROWS becomes a form method, plus is_line_text_row, line_editor, move_chip, content_index, row_count, render_create layout, and open_edit_focus. Deliberately deferred from the family release to avoid touching the core create/edit form under time pressure; the family core goal is met without it.
