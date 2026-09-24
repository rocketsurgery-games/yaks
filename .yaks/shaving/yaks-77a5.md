---
id: yaks-77a5
title: 'Case study: reconstruct the 2026-09-23 UI lightning round (11 lanes + 3 scouts)'
type: task
priority: 3
created: '2026-09-24T01:30:48Z'
updated: '2026-09-24T01:33:44Z'
parent: yaks-64c4
labels:
- ui
---

Post-facto archaeology of a large parallel run, to see what's recoverable and what's lost. Step 1: capture the coordinator's conversation-only context before it's lost (lane SHAs, agent durations, merge/conflict log). Step 2: document the gaps. Step 3: sketch a timeline. Then decide what tooling would help.

![lightning-2026-09-23-context](artifacts/yaks-77a5/lightning-2026-09-23-context.md)

![gaps](artifacts/yaks-77a5/gaps.md)

![timeline](artifacts/yaks-77a5/timeline.md)

![swimlane](artifacts/yaks-77a5/swimlane.txt)

![swimlane](artifacts/yaks-77a5/swimlane.py)

---
▸ 2026-09-24T01:33:44Z [coordinator]
Steps 1–3 done. (1) Conversation-only context captured in lightning-2026-09-23-context.md: star input set, triage, the claim hiccup, spawn time, per-agent duration/tools/tokens, lane→squash SHA map, merge + conflict log, the shared-target friction. Lane commits pinned at refs/archive/lightning-2026-09-23/lane-* (LOCAL ONLY, not pushed). (2) gaps.md: 10 gaps ranked by how much is lost, each with candidate fixes. The top 3: lane SHAs lost to squash + branch delete; agent lifecycle exists only in the harness; integration fixes invisible. (3) timeline.md + swimlane.txt/.py: 10m29s claim→last merge, ~6.5x effective parallelism, the merge queue as the end-of-run bottleneck. Next: decide which capture conventions and tooling to adopt (a skill convention vs a yaks timeline view).
