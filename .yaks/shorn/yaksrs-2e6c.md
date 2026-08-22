---
id: yaksrs-2e6c
title: Help (?) doesn't do anything
type: bug
priority: 3
created: '2026-08-22T19:05:14Z'
updated: '2026-08-22T19:23:20Z'
parent: yaksrs-0a93
---

At the least, it needs the basic keyboard help. We can reuse the right-side panel (and add overflow/scrolling) rather than the Python version's overlay dialog.

---
▸ 2026-08-22T19:23:20Z
Added Overlay::Help — a scrollable keyboard reference in the right pane (shares right_divider), opened by ? from both list and detail panes. help_content() lists the actual Rust bindings by section (Movement/List/Detail/Edit/Search & filter/General); j/k/d/u/g/G scroll (clamped to detail_page), ?/q/Esc close. Status-line hint while open. Reused the right-side panel per the yak's direction rather than Python's centered popup. Tests: help_overlay snapshot + help_opens_and_closes. 106 unit + 19 CLI green, warning-free.
