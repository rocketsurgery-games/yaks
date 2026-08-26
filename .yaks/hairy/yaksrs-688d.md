---
id: yaksrs-688d
title: Informal yak-linking affordances
type: feature
priority: 3
created: '2026-08-26T13:32:33Z'
updated: '2026-08-26T13:32:33Z'
labels:
- ui
- links
---

While we have formal parentage and dependencies among yaks, sometimes it's useful just to be able to mention a yak in a desc/comment and link to it.
We could require formal `[yak-123]` link structures to make it more markdown-friendly, or just detect the `yak-123` pattern with the configured prefix.
Then we can add the ability to follow them like any other link, and autocompletion during editing, which would save a lot of copying/pasting/memory.
