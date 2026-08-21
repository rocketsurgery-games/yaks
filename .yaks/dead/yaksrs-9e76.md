---
id: yaksrs-9e76
title: 'TUI slice 3c: edtui full-body edit (E)'
type: task
priority: 3
created: '2026-08-21T00:08:24Z'
updated: '2026-08-21T00:41:17Z'
parent: yaksrs-86a3
labels:
- rust
- phase2
---

Add the edtui dependency and an in-frame multi-line editor overlay for E (edit selected task body/description). Vim + emacs profiles per config; no terminal takeover. Commit -> Herd::update{description}; Esc -> cancel. Keep render pure (editor state lives in App). This is the first dep add since slice 1; verify warning-free build + snapshot the editor overlay.

---
▸ 2026-08-21T00:41:17Z
Merged into yaksrs-1699 (single shared edtui editor overlay). No separate hand-rolled line editor.
