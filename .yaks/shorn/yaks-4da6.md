---
id: yaks-4da6
title: Text wrapping bugs on single-line inputs
type: bug
priority: 3
created: '2026-09-07T02:40:09Z'
updated: '2026-09-23T21:42:56Z'
labels:
- ui
---

When a single-line input (eg, "title") overflows its available space, it wraps to the next line. But the second line of text doesn't render because it's obscured by the following line (though you can type and see the cursor position).

We should decide what to do in this scenario -- I think horizontal scrolling would be best/simplest, and still nudge the user to keep single-line inputs reasonably terse.

---
▸ 2026-09-23T21:35:37Z [coordinator]
Lightning-round lane 'form' (worktree wt/form, branch lane-form). Scope: single-line inputs in create/edit form (create.rs + form render): horizontal scroll. Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![lane-form-title-hscroll](artifacts/yaks-4da6/lane-form-title-hscroll.txt)

---
▸ 2026-09-23T21:42:48Z [lane-form]
Headless frame (72x14): create form with a title longer than the field; it scrolls horizontally so the tail + cursor (END) stay visible and the type row below is intact. Snapshot: create_form_long_title_scrolls_horizontally.

---
▸ 2026-09-23T21:42:56Z [lane-form]
Shorn: single-line edtui fields now render with .wrap(false) (render_text_row in render.rs — create/edit form + filter drawer text rows; render_query_line in editor.rs — search/fuzzy/line editors), so an overflowing value scrolls horizontally keeping the cursor visible instead of wrapping under the next row. Unfocused rows show the head, truncated. Regression test create_form_long_title_scrolls_horizontally (+ snapshot). docs/tui.md notes it. Gate: cargo test -p yaks green (284 unit + 25 cli).
