---
id: yaks-b3a0
title: 'edtui PR: ~ toggle case action'
type: feature
priority: 3
created: '2026-08-24T22:11:59Z'
updated: '2026-08-25T22:47:17Z'
parent: yaks-6099
labels:
- ui
- edtui
- upstream
---

Add a ToggleCase Action + default vim binding for ~ (flip case of char under cursor, advance). Reference impl: yaks toggle_case() (via ReplaceChar so it's undoable). Requires GitHub fork.

---
▸ 2026-08-25T04:00:02Z
Note: the ~ (toggle case) BEHAVIOR already ships via a yaks-side shim (toggle_case in route_multiline_key) and works today. This yak now only tracks the OPTIONAL upstream relocation into edtui (deferred, issues-first). Not blocking anything.

---
▸ 2026-08-25T22:47:17Z
Done natively: added ToggleCase (~) to edtui on branch yaks/toggle-case, merged into yaks-integration. yaks now consumes it and the yaks-side ~ shim was DELETED. Undoable + count-aware + dot-repeatable. Ready for you to review the branch and upstream issue-first.
