---
id: yaksrs-f2cf
title: 'Detail pane: emoji status glyphs in deps/blocks/parent/children (match list)'
type: task
priority: 4
created: '2026-08-22T15:13:43Z'
updated: '2026-08-22T15:53:40Z'
parent: yaksrs-0a93
labels:
- rust
- tui
- parity
---

detail.rs ref_line uses status.glyph() which renders letters (H/N/S); the list pane + tab bar use emoji (🦬/🪒/🐑). For consistency, detail sections should use the same emoji glyph. Low priority polish surfaced while doing 425c.

---
▸ 2026-08-22T15:53:40Z
detail.rs ref_line now takes a &str glyph and uses Status::emoji() for deps/blocks/parent/children, matching the list + tab bar. Added Status::emoji() to model.rs; deduped tui.rs status_emoji() to delegate. Link offsets remain char-indexed (render_dline styles as relative span flow) so the width-2 emoji doesn't skew id highlighting. Updated 2 detail insta snapshots (H a1 -> emoji). 100 unit + 19 CLI green, warning-free.
