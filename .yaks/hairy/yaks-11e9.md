---
id: yaks-11e9
title: 'a3a6 TUI: inbox as a filter/saved-view; remove the i toggle'
type: feature
priority: 3
created: '2026-09-03T22:28:44Z'
updated: '2026-09-03T22:28:44Z'
parent: yaks-a3a6
labels:
- ui
---

After the prep lands: use FilterSpec's needs predicate for an 'inbox' saved view (like 'recent') and/or a filter usable in any view; remove the App.inbox_only modal toggle from yaks-548b. Lane scope: src/tui.rs + src/tui/*.rs only.
