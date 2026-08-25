---
id: yaksrs-c0d9
title: Text blocks in details don't wrap properly
type: bug
priority: 1
created: '2026-08-23T05:02:08Z'
updated: '2026-08-25T02:25:12Z'
labels:
- ui
---

They just run off the right edge, until you move into edit mode, at which point they wrap while you're editing.

In edit mode, the wrapping doesn't break words, and there appears to be no way to move vertically within long, wrapped lines ('gu', 'gd', 'g0', 'g$' in vim).

---
▸ 2026-08-25T02:25:12Z
Fixed the primary complaint: the read-only detail pane now soft-wraps long text (word boundaries, hard-break only for over-long words) instead of running off the right edge. Approach: detail::wrap() pre-wraps the DLine list into physical rows and remaps each link (col,len) onto its row, so the row-indexed model (line cursor, jumplist, find, scroll, visual select) keeps working with 1 DLine = 1 screen row. Width captured at render into App.detail_width (Cell); width 0 = no-op. Added cont flag to DLine so yank rejoins soft-wrapped rows into their logical line (no hard breaks in copied text). All detail consumers (detail_jumps/line_count/find_matches/yank/render) now go through App::detail_dlines(). Tests: 4 wrap unit tests + detail_body_wraps_at_narrow_width e2e; workspace green (125), 0 warnings. The other half of this yak's notes (vertical nav in wrapped lines) was already delivered as gj/gk/g0/g$ under 6099. Still open: the EDITOR (edit mode) hard-wraps mid-word (edtui LineWrapper is a char-wrap) - that's an edtui concern, spun out.
