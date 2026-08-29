---
id: yaks-ffae
title: 'Parity: M adds a comment (timestamped note) to a yak'
type: task
priority: 3
created: '2026-08-22T20:29:49Z'
updated: '2026-08-22T20:30:51Z'
parent: yaks-0a93
labels:
- rust
- tui
- parity
---

Python list+detail M opens a multi-line editor and appends a '---/ ▸ {iso} / text' note block to the body. Rust already has this via TaskEdit.note (store::append_note). Add EditAction::Comment + open_comment (multi-line Overlay::Edit) bound to M in both panes.

---
▸ 2026-08-22T20:30:51Z
Added EditAction::Comment + open_comment: M opens a multi-line editor (reusing the Overlay::Edit infra kept from 3b22) and on Ctrl-S appends a timestamped note via TaskEdit.note (store::append_note). Bound in both panes; added to ? help. Test comment_appends_timestamped_note. 111 unit + 19 CLI green.
