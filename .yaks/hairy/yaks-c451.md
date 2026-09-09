---
id: yaks-c451
title: Animated TUI doc examples via SMIL (SVG animation)
type: idea
priority: 4
created: '2026-09-06T23:25:51Z'
updated: '2026-09-08T23:09:07Z'
labels:
- docs
- ui
---

Followup to the buffer->SVG renderer (yaks-3677): drive the headless TUI through a keystroke SEQUENCE, render a frame per step, and stitch them into ONE animated SVG using SMIL (<animate>/<set> toggling per-frame layers via visibility/opacity, keyTimes over a loop). Gives self-contained, dependency-free animated 'screencasts' that render in a browser/markdown. Joel's idea; deferred. Consider frame-diffing to keep size down.
