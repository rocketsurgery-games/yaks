---
id: yaksrs-afb0
title: Tabbing through links in details always moves the selected link to the top line of the viewport
type: bug
priority: 3
created: '2026-08-22T19:08:37Z'
updated: '2026-08-22T19:14:05Z'
parent: yaksrs-0a93
---

It should at least keep it in the center so you can see context. Better if it can just ensure that it's scrolled into view, so that the scroll position doesn't change every time you select a link.

---
▸ 2026-08-22T19:14:05Z
jump_link (Tab/[ ]) now calls scroll_line_into_view instead of snapping detail_scroll to the target line. Added App.detail_page (detail viewport height = terminal height - 3), set in the event loop and headless driver. scroll_line_into_view moves the scroll the minimum needed to reveal the line and leaves it untouched when already visible, so cycling links no longer jerks the viewport. Unit test scroll_into_view_is_stable_and_minimal. 101 unit + 19 CLI green.
