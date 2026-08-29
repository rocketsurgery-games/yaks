---
id: yaksrs-15f7
title: Allow the TUI to handle multiple .yaks dirs
type: idea
priority: 3
created: '2026-08-27T22:51:04Z'
updated: '2026-08-29T18:37:56Z'
parent: yaksrs-688d
depends_on:
- yaks-0187
---

This could be really useful when working across multiple project repos, especially if they have different upstream issue trackers.
On the yaks side, we could use this unification to manage cross-cutting concerns, especially useful when a single agent needs to make changes to both projects.
Each .yaks would still get its own config and yak prefix. We could also allow cross-references, though that might require an explicit "friend" relationship to be modeled in the config, so that the tools know where to find all the yak files.

---
▸ 2026-08-29T18:37:56Z
This is the federation layer, and it sets a hard requirement on the core (yaks-0187): the resolver must take the id set as a parameter so it can widen from one herd to a friend-set without changing callers. Cross-herd refs need a herd-qualified reference form (reserved in 0187 grammar). Consequence for rename (7a92): prefixes must be UNIQUE across the friend-set or a qualified ref is ambiguous — which loops straight back to reconciling the yaks/yaksrs split. "Friend" relationship modeled in each config so tools know where to find sibling yak files.
