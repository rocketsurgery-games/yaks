---
id: yaks-685e
title: 'TUI: accent styling for the needs block (detail line + badge)'
type: feature
priority: 3
created: '2026-09-04T04:23:24Z'
updated: '2026-09-04T04:23:24Z'
parent: yaks-594b
labels:
- ui
---

Follow-up deferred from yaks-4e8a/548b: give the needs block a warning accent so it reads as a blocker. In src/tui/detail.rs, the 'Needs:' field (added by 4e8a) renders as a plain Kind::Field; give it an accent (a new DLine Kind, e.g. Kind::Warn, threaded through render_dline's style match — mirror how Kind::Section is colored). Optionally tint the '⏳' needs badge in the list rows (tui.rs list_item) to match. Scope: src/tui.rs + src/tui/detail.rs only. Keep it minimal; regenerate any affected snapshots.
