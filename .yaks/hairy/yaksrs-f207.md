---
id: yaksrs-f207
title: Changing priority (or anything that causes a list resort) in detail mode doesn't update selection
type: bug
priority: 3
created: '2026-08-23T18:26:53Z'
updated: '2026-08-23T18:26:53Z'
labels:
- ui
---

Presumably the selection model is index-based, so when you change a property that updates the current yak's sort order, the rendering updates to point at whatever yak is *now* in its slot. Would be nice if it actually followed the new sort position (if it's still in the current list). If it drops *out* of the list, we should probably just close the detail panel (or leave it up, unchanged, and no longer tracking a list position; if that's a thing the UI structure can represent).
