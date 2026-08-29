---
id: yaksrs-9aac
title: Search input has no normal mode
type: bug
priority: 3
created: '2026-08-28T05:09:17Z'
updated: '2026-08-28T05:09:17Z'
labels:
- ui
---

Hitting escape once, even when vi mode's enabled, drops you immediately out of search. It should require Ctrl-C/EscEsc like other one-line editors.

Also, when you hit / again after completing one search, it opens with the text unselected, in insert mode, on the first character. This is quite
confusing, because you just start typing, but what you typed ends up as a prefix on the last search term. I believe it's more common to either start
it fresh each time (vi), or pre-select the text in insert mode (most structured UIs), so that if you just start typing, it replaces the text.
