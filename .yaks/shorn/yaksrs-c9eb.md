---
id: yaksrs-c9eb
title: Need line selection / copy in yak detail pane
type: feature
priority: 3
created: '2026-08-22T20:13:39Z'
updated: '2026-08-22T20:48:05Z'
parent: yaksrs-0a93
---

The python impl has this -- you can select lines, and v-select multiples, to copy blocks.

---
▸ 2026-08-22T20:48:05Z
Adopted Python's detail line-cursor model. Replaced detail_link (jumplist index) with detail_line (per-line cursor) + detail_anchor (visual selection). j/k/d/u/g/G now move the line cursor (auto-scroll via scroll_line_into_view); Tab/Shift-Tab snap it to link lines; Enter follows the link on the cursor line. v toggles visual selection; Shift-arrows extend; y/Enter copy the selected line block (dedented) to the clipboard; Esc peels back selection->find->list. render_dline gained a line_bg param; the cursor line + selection get bg idx237 (verified via --style). Added dedent(). Tests: detail_tab_cycles_link_lines, detail_visual_selection_and_esc_clears; updated the 3 follow-link tests to Tab-then-Enter. 112 unit + 19 CLI green, no new clippy.
