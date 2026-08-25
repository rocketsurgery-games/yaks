---
id: yaksrs-cdf9
title: 'edtui PR: x deletes into the clipboard'
type: feature
priority: 3
created: '2026-08-24T22:12:10Z'
updated: '2026-08-24T22:12:10Z'
parent: yaksrs-6099
labels:
- ui
- edtui
- upstream
---

RemoveChar (x) currently doesn't populate state.clip, so x then p doesn't work like vim. Make RemoveChar yank the removed char(s) via delete_range/clip (dd/dw already do). Small, matches vim. Requires GitHub fork.
