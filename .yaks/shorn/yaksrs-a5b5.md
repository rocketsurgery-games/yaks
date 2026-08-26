---
id: yaksrs-a5b5
title: Unfocused description in edit/create form loses word-wrapping
type: bug
priority: 2
created: '2026-08-26T02:40:40Z'
updated: '2026-08-26T02:42:25Z'
labels:
- ui
---

In the edit/create form, the description field wraps correctly when focused (edtui word-wrap) and in the detail view pane, but the dimmed unfocused preview renders via a plain Paragraph without .wrap(), so long lines get truncated at the pane edge instead of wrapping. Fix: give the unfocused preview the same soft-wrap so it reads the same whether focused or not.

---
▸ 2026-08-26T02:42:25Z
Fixed: the unfocused description preview in render_create used a plain Paragraph with no .wrap(), so long lines truncated at the pane edge. Added .wrap(Wrap { trim: false }) (trim:false preserves leading indentation) so the dimmed preview soft-wraps like the focused editor and detail view. Added regression test unfocused_description_preview_wraps.
