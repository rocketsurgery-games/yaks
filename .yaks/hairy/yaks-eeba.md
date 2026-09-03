---
id: yaks-eeba
title: Render preserved unknown frontmatter fields in TUI detail
type: feature
priority: 3
created: '2026-09-03T19:58:25Z'
updated: '2026-09-03T19:58:25Z'
parent: yaks-594b
labels:
- ui
---

TUI counterpart to the CLI show render: surface Task.extra (unmodeled frontmatter kept by yaks-031d) read-only in the detail pane, so hand-added/newer keys are visible instead of silently invisible. A small 'Other fields:' section near the end of the header block. Scope: src/tui/detail.rs (build). Read-only.
