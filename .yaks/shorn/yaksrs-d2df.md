---
id: yaksrs-d2df
title: Rust implementation needs auto-refresh
type: bug
priority: 3
created: '2026-08-22T15:19:24Z'
updated: '2026-08-22T19:47:22Z'
parent: yaksrs-0a93
---

However the python code was working, the new implementation really needs to watch the filesystem, or at least just check regularly or on operations, so that the UI state doesn't drift relative to the on-disk state. That risks serious concurrent-edit bugs.

---
▸ 2026-08-22T19:47:22Z
Added a recursive notify (6.1.1) file-watcher on the herd .yaks/ tree, wired into the event loop. Loop now blocks on input via event::poll(250ms); content-changing fs events (create/remove/data-modify; access+metadata filtered to avoid a read->reload loop) set a pending-refresh flag applied by reload_preserving_selection ONLY while idle (overlay==None), so external edits never yank data from under an open editor/picker. Cursor kept on the same task by id; refresh is silent (doesn't clobber mutation notifications). Best-effort: watcher failure degrades gracefully (no auto-refresh). Unit test reload_preserving_selection_picks_up_external_add. 109 unit + 19 CLI green, no new clippy. docs/tui-parity.md #13.
