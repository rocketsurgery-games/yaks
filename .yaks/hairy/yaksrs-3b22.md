---
id: yaksrs-3b22
title: 'Parity: E opens a full edit form (share the create-form widget)'
type: task
priority: 3
created: '2026-08-22T16:03:41Z'
updated: '2026-08-22T16:03:41Z'
parent: yaksrs-0a93
labels:
- rust
- tui
- parity
---

Python's E reopens the whole task form (title/type/priority/labels/description) in edit mode; Rust's E edits only the description body (other fields via L/P/T/S). Now that the create form exists (d13b: Overlay::Create/CreateForm), refactor it into a shared create/edit widget seeded from an existing task, and route E through it. Aligns with the goal of a single shared editor implementation. See docs/tui-parity.md #8 and #12. Surfaced by a083.
