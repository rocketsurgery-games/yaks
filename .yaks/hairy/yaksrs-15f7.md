---
id: yaksrs-15f7
title: Allow the TUI to handle multiple .yaks dirs
type: idea
priority: 3
created: '2026-08-27T22:51:04Z'
updated: '2026-08-27T22:51:04Z'
---

This could be really useful when working across multiple project repos, especially if they have different upstream issue trackers.
On the yaks side, we could use this unification to manage cross-cutting concerns, especially useful when a single agent needs to make changes to both projects.
Each .yaks would still get its own config and yak prefix. We could also allow cross-references, though that might require an explicit "friend" relationship to be modeled in the config, so that the tools know where to find all the yak files.
