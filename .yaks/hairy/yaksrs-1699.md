---
id: yaksrs-1699
title: 'TUI slice 3b: line-prompt overlay + labels (L) + create (c/C)'
type: task
priority: 3
created: '2026-08-21T00:08:18Z'
updated: '2026-08-21T00:08:18Z'
parent: yaksrs-86a3
labels:
- rust
- phase2
---

Text-input mutations. Add a single-line prompt overlay (a small line editor reproducing Python edit_prompt/input_prompt: cursor, backspace, left/right, Enter=commit, Esc=cancel). Wire L labels (comma-split edit of current labels -> update add/remove), c create (prompt title, then type picker t/b/f/i, create as root hairy), C create-child (same, parent = selected id). Reload + notification after each. Snapshot the prompt overlay and a created row.
