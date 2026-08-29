---
id: yaks-d13b
title: 'Parity: create as a modal form (title/type/priority/labels/description)'
type: task
priority: 3
created: '2026-08-22T03:44:55Z'
updated: '2026-08-22T15:49:16Z'
parent: yaks-0a93
labels:
- rust
- tui
- parity
---

Python create is a full-screen form (Tab/j/k move, ←→ pick chips, Enter edit, (need title) hint, Esc cancel). Rust uses sequential bottom prompts. Build the form. docs/tui-parity.md #8.

---
▸ 2026-08-22T15:49:05Z
Built right-pane Overlay::Create form (title/type/priority/labels/description). Single-select chip rows (cursor==value); Tab/arrows/j/k nav; Enter creates with (need title) guard; Esc cancels. Generalized render_chip_row/render_text_row to take current_row/cursor_idx/label_w/placeholder (drawer LABEL_W=9, create=12); removed old EditAction::CreateTitle + PickAction::CreateType two-step chain. NewTask now carries priority+labels+description. Tests: renamed create_title_field->create_form insta snap; rewrote live create tests + added priority/labels + empty-title-noop cases. 100 unit + 19 CLI green, build warning-free.
