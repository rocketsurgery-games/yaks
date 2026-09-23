---
id: yaks-28b4
title: Keep detail cursor/scroll position on back/forward nav
type: bug
priority: 3
created: '2026-09-07T02:38:01Z'
updated: '2026-09-23T21:41:38Z'
labels:
- ui
---

When navigating forward and backward through yak detail links, we lose the cursor/scroll position. We should preserve this as part of the stack.
Relates to yaks-158d; consider implementing together.

---
▸ 2026-09-23T21:35:38Z [coordinator]
Lightning-round lane 'navpos' (worktree wt/navpos, branch lane-navpos). Scope: preserve detail cursor/scroll in the detail nav stack (detail_nav.rs). Evidence contract: gate = cargo test -p yaks green (re-accept + eyeball any snapshot ripple); PLUS drive the headless TUI (toque) to the relevant state and yaks attach the frame (text, or docshot SVG when color matters). Judge: coordinator at merge review; subjective forks -> yaks ask and hand back.

![navpos-back-forward-frames](artifacts/yaks-28b4/navpos-back-forward-frames.txt)

---
▸ 2026-09-23T21:41:10Z [lane-navpos]
Headless 80x16 text frames from nav_history_restores_cursor_and_scroll: a0 scrolled to its a1 link; a1 after follow + j x20; o (back) renders a0 byte-identical to how it was left (notification row blanked).

---
▸ 2026-09-23T21:41:38Z [lane-navpos]
Done. o/i history entries are now a small NavEntry {id, line, scroll} (src/tui.rs) instead of a bare id; follow_link/nav_back/nav_forward snapshot the current detail cursor+scroll before pushing, and restore_nav_entry reopens the target and puts cursor/scroll back (clamped to the task's current line count). Local precursor to yaks-158d (global cross-view history) — NavEntry is the natural seed for that entry type. Test nav_history_restores_cursor_and_scroll; docs/tui.md updated. cargo test -p yaks --bins: 282 passed. Observation (not fixed): detail_page = h-3 counts the sticky title row, so a Tab-to-link at the bottom can land one row below the visible viewport (visible in the attached a0 frame: link line just off-screen).
