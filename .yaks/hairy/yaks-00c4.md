---
id: yaks-00c4
title: Affordance for auto-creating a yak from an upstream issue
type: feature
priority: 3
created: '2026-08-27T16:30:57Z'
updated: '2026-08-27T16:30:57Z'
---

Both for CLI and in a TUI shortcut -- given an upstream issue URL, auto-create a yak from it.
As a starting point, I think it would need to just copy the title and (perhaps?) content to a yak, because we don't have (or want) an LLM in the tool.
We might consider a marker (tag, or text) signifying that it has just been copied, and could use tidying. Then the tool could signal to your agent that it should do the tidying when ready.
