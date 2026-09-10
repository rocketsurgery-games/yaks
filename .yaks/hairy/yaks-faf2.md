---
id: yaks-faf2
title: Ease visual review of attached SVGs (SVG->PNG render helper)
type: idea
priority: 3
created: '2026-09-10T23:45:35Z'
updated: '2026-09-10T23:45:35Z'
labels:
- meta
---

Coordinating the UI batch, the coordinator cannot cheaply eyeball an attached SVG: agent read_file is in-project only and cannot rasterize SVG, so visual review needs an out-of-band headless-Chrome SVG->PNG dance into a gitignored dir. A helper -- yaks attach --render, a standalone yaks render <svg>, or at least a documented skill snippet -- that drops a PNG beside the artifact would smooth the --ask visual-review loop that TUI yaks now depend on.
