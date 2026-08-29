---
id: yaks-425c
title: 'Parity: detail pane (Task: header, capitalized labels, dates, Blocks/reverse-deps, Parent/Children)'
type: task
priority: 3
created: '2026-08-22T03:44:55Z'
updated: '2026-08-22T15:13:43Z'
parent: yaks-0a93
labels:
- rust
- tui
- parity
---

Match Python detail: 'Task: {id}' header; Title:/Status:/Type:/Priority:/Created:/Updated:/Labels: (capitalized, ~13 pad); humanized dates; Depends on:/Blocks:(reverse deps)/Parent:/Children: sections. Rust currently lacks Status/dates/reverse-deps. docs/tui-parity.md #7.

---
▸ 2026-08-22T15:13:43Z
Done: rewrote detail::build to match Python parity (doc §7). Task: header; capitalized 13-wide Title/Status/Type/Priority/Created/Updated/Labels; humanize_date port (Aug 22, 2026 03:44); conditional Depends on / Blocks (reverse deps) / Parent / Children sections. 98 unit + 19 CLI tests; regenerated 2 detail snapshots. Nit filed: emoji glyphs in detail sections.
