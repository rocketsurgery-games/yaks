---
id: yaks-e741
title: 'Core ops facade (Herd): thin CLI over reusable core (TUI/plugin prep)'
type: task
priority: 2
created: '2026-08-20T22:31:13Z'
updated: '2026-08-20T22:37:42Z'
labels:
- rust
- ui
---

Refactor per yaks-9b82. Introduce a print-free core::Herd facade that owns every operation as a typed method (create/update/transition/dep/reparent/list/next/tangled/search/stats/show/rollup), composing store+filter+rollup. Herd::open does discovery + the schema gate (returns typed OpenError; carries an older-schema warning). main.rs becomes parse -> Herd op -> render (text/--json); no logic in handlers. Keep core clap/render-free so it can extract to a yaks-core lib crate later (for a persistent LSP-like process serving IDE plugins). Behavior must stay byte-identical — the 35 tests (incl. 17 golden snapshots) are the safety net.

---
▸ 2026-08-20T22:37:42Z
Done. herd::Herd — print-free core ops facade with whole-operation typed methods (open/list/next/tangled/search/stats/show/rollup/create/update/transition/dep_add/dep_remove/reparent). main.rs is now parse -> Herd op -> render only; schema gate moved into Herd::open (typed OpenError + schema_warning). rollup::Group is owned so groups outlive the load; json renders from typed results. Behavior byte-identical: 35 tests pass (17 golden snapshots unchanged), warning-free. herd is clap/render-free, ready to extract to a yaks-core lib crate for a future LSP-like process.
