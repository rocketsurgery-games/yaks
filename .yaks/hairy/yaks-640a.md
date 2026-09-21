---
id: yaks-640a
title: Explicit label definitions in config.yaml
type: feature
priority: 3
created: '2026-08-23T02:49:59Z'
updated: '2026-09-21T18:37:07Z'
labels:
- config
---

While the yaks CLI/TUI doesn't need explicit label definitions for anything, it could still be useful for auto-complete in the UI.
More importantly, if we can find a good way to get context into the calling agent (cf yak-011e), we can use label definitions to help nudge it in the right direction.

---
▸ 2026-09-21T18:37:07Z [agent]
Multi-herd angle: with the farm config herds: map + one-level global->herd cascade (yaks-98d5), label definitions should be per-herd-overridable (project A's labels differ from B's in a consolidated farm). Resolve label defs through that cascade rather than a single global list.
