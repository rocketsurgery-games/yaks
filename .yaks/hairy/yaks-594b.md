---
id: yaks-594b
title: Surface needs + attribution across the TUI/CLI
type: task
priority: 2
created: '2026-09-03T17:58:45Z'
updated: '2026-09-03T17:58:45Z'
parent: yaks-3901
labels:
- agent
- ui
---

eb66 (actor attribution) and b517 (needs-human block) landed at the FILE + CLI-command level, but the read/query/render surfaces lag. Dogfooding found the model works on disk while the UI is blind to it (Joel: 'yaks ask yaks-b517 did the right thing at the file level, but there are no UI affordances to render/query it'). Umbrella for the affordances that make needs + attribution first-class in both surfaces. Children are the concrete gaps; each is a small, independently shippable slice.
