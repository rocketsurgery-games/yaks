---
id: yaksrs-d416
title: 'Parity: filter drawer as a top drawer with [x] checkboxes'
type: task
priority: 3
created: '2026-08-22T03:44:55Z'
updated: '2026-08-22T15:27:20Z'
parent: yaksrs-0a93
labels:
- rust
- tui
- parity
---

Python drawer sits above the list (list still visible) with [x]/[ ] checkbox chips. Rust drawer is in the right pane with highlighted chips. Move to a top drawer with checkboxes. docs/tui-parity.md #9.

---
▸ 2026-08-22T15:27:20Z
Done (reframed vs Python): filter drawer + fuzzy/view pickers + multiline editor stay in the RIGHT pane (intentional divergence from Python's top drawer - better for wide/short terminals) and now share the detail pane's left divider via a new right_divider() helper, so they no longer blend into the list. Also fixed the detail dim-label prefix 9->13 to match the wider fields. Chip style kept (▸ + green highlight), not Python [x] checkboxes. Regenerated 5 overlay snapshots; 98 unit + 19 CLI tests, warning-free.
