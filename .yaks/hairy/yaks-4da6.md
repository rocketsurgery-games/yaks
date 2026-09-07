---
id: yaks-4da6
title: Text wrapping bugs on single-line inputs
type: bug
priority: 3
created: '2026-09-07T02:40:09Z'
updated: '2026-09-07T02:40:09Z'
labels:
- ui
---

When a single-line input (eg, "title") overflows its available space, it wraps to the next line. But the second line of text doesn't render because it's obscured by the following line (though you can type and see the cursor position).

We should decide what to do in this scenario -- I think horizontal scrolling would be best/simplest, and still nudge the user to keep single-line inputs reasonably terse.
