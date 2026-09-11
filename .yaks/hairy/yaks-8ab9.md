---
id: yaks-8ab9
title: b1cc/1 Scaffolding + shared test_support
type: task
priority: 3
created: '2026-09-11T01:29:48Z'
updated: '2026-09-11T01:29:48Z'
parent: yaks-b1cc
labels:
- ui
verify: cargo test -p yaks
---

Foundation, no logic moves. Introduce src/tui/test_support.rs (cfg(test), pub(crate) helpers: draw/sample/editable/linked + the 'live' harness temp_herd/press), and rewire tui.rs mod tests to use it, so later phases can relocate tests and reuse helpers. Green + snapshots unchanged.
