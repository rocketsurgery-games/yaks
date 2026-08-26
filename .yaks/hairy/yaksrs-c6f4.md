---
id: yaksrs-c6f4
title: Window/scroll the edit-form content accordion when blocks overflow
type: task
priority: 4
created: '2026-08-26T12:53:40Z'
updated: '2026-08-26T13:35:05Z'
labels:
- ui
- editing
---

Follow-up to the block accordion in the edit form (yaksrs-127e). With many comments, the fixed 1-line separators above the focused block can push the focused block's editor off the bottom of a short pane (the single Min(0) editor gets little/no height and later separators clip). Confirmed reproducible on a task with enough comments.

Fix: window or scroll the content stack so the focused block stays fully visible -- e.g. render only a range of blocks around the focused one, or offset the stack so the focused separator + its editor are in view, with a small indicator (…) when blocks are hidden above/below. Low priority: typical comment counts render fine; this only bites with long comment stacks on short viewports.
