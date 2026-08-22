---
id: yaksrs-f2cf
title: 'Detail pane: emoji status glyphs in deps/blocks/parent/children (match list)'
type: task
priority: 4
created: '2026-08-22T15:13:43Z'
updated: '2026-08-22T15:13:43Z'
parent: yaksrs-0a93
labels:
- rust
- tui
- parity
---

detail.rs ref_line uses status.glyph() which renders letters (H/N/S); the list pane + tab bar use emoji (🦬/🪒/🐑). For consistency, detail sections should use the same emoji glyph. Low priority polish surfaced while doing 425c.
