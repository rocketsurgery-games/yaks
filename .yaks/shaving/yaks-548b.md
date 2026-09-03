---
id: yaks-548b
title: 'TUI needs affordances: list badge, inbox filter/view, ask/answer keybindings'
type: feature
priority: 3
created: '2026-09-03T17:59:19Z'
updated: '2026-09-03T20:02:52Z'
parent: yaks-594b
depends_on:
- yaks-4e8a
- yaks-1edb
labels:
- ui
---

Make needs actionable in the TUI: (1) a list-row badge/indicator for needs-blocked yaks (parallels the CLI row marker); (2) a filter/saved-view for the inbox (yaks awaiting a human); (3) keybindings to ask (block + prompt for a question) and answer (clear + prompt for a reply) from the detail/list, reusing the CLI verbs. Builds on detail rendering (yaks-4e8a) and the comment-actor work (yaks-1edb).
