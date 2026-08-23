---
id: yaksrs-2c16
title: CLI/MCP affordance to attach to a running yaks tui
type: feature
priority: 3
created: '2026-08-23T02:49:59Z'
updated: '2026-08-23T02:49:59Z'
labels:
- cli
---

Carried over from Python-repo yak-ea69. Feeds the IDE-plugin / long-lived-process vision: a way to attach to or drive a running yaks tui over a socket/named-pipe, so an editor plugin can reuse the core logic instead of reimplementing it. Relates to the thin-UI-over-core architecture.
