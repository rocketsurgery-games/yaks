---
id: yaksrs-fe4f
title: Add :q and :wq affordances for vim-friendliness
type: task
priority: 3
created: '2026-08-26T12:47:52Z'
updated: '2026-08-27T16:41:32Z'
parent: yaksrs-fc85
labels:
- ui
---

Seems like a lot for such a simple thing, but muscle-memory dies hard!
Perhaps we can leave a spot for general :command affordances as part of this, even if we don't add many.

---
▸ 2026-08-27T16:41:32Z
Implemented a minimal vim command line. In a multiline editor's Normal mode (comment M + form content rows), ':' opens a command line in the status bar (App.cmdline); Enter runs, Esc dismisses, Backspace edits. Commands: :w / :wq / :x commit (a modal editor saves+closes, so all three behave alike), :q cancels (respecting the 275f dirty-discard confirm), :q! force-cancels. Unknown verbs -> 'unknown command: :X' and the editor stays open. run_command's catch-all is the extension point for future : verbs. Scope: multiline surfaces only (single-line fields don't get ':'). Help updated. Tests: :wq commits, :q! discards, :w saves the form, unknown notifies. 158 bin green, 0 warnings.
