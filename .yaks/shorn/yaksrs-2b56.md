---
id: yaksrs-2b56
title: 'Parity: list full-width in list focus, split only in detail'
type: task
priority: 3
created: '2026-08-22T03:44:40Z'
updated: '2026-08-22T04:03:29Z'
parent: yaksrs-0a93
labels:
- rust
- tui
- parity
---

Python shows a full-width list in list mode and only splits into list+detail when you press l. Rust is always two-pane. Match: hide the detail pane when focus=list; split when focus=detail. See docs/tui-parity.md #1.

---
▸ 2026-08-22T04:03:29Z
Done. render() now: tabs row, blank gap row, main, help bar. List is full-width when focus=list; focus=detail splits list(34%)+detail(66%). Interim: right-pane overlays (multiline edit/fuzzy/drawer/view-picker) still borrow the split until relocated (top drawer / full-screen pickers) with their own yaks.
