---
id: yaksrs-2c16
title: CLI/MCP affordance to attach to a running yaks tui
type: idea
priority: 3
created: '2026-08-23T02:49:59Z'
updated: '2026-08-26T13:17:10Z'
labels:
- cli
---

It would be *really* useful for your agent to have a way to drive the yaks tui you have open. Eg, "(user) Find me all the hairy yaks with the 'ui' label", or "(agent) Let me show you the yak herd I just created so you can review", and so forth.
We'll need to think about the mechanics of this, as it's not entirely obvious how best to approach it. Eg:
- What if you have multiple TUIs running? Are they named somehow?
- Can the agent query them all to figure out which one to connect to?
- What happens if multiple agents try to connect to the same one?
- And so on...

As we work through the mechanics and use-cases, let's make a point to identify UI affordances that would make this more effective. Some rough ideas:
- Some affordance for saved searches, allowing the agent to show you a search without interrupting whatever you were working on.
- Ephemeral agent-specific UI state -- relating to saved searches, a way for an agent to push a new UI state on the stack, so the user can just ESC out of it, back to where they were.
- A way to "indicate" to the agent that you want it to look at a particular yak/herd/search/comment/... (we handle this for individual yaks with the ability to copy an id).
