---
id: yaks-5fae
title: Bulk and filtered field updates & reparents
type: feature
priority: 2
created: '2026-08-23T03:43:29Z'
updated: '2026-09-04T17:17:52Z'
parent: yaks-8d53
labels:
- cli
---

update and reparent accept multiple ids, and optionally a filter selector to apply across a matching set (e.g. bulk relabel every yak with a given label, or reparent a whole subtree). This is the core moving-and-refactoring affordance. Needs a design decision on how to express the selector safely.

---
▸ 2026-09-04T17:17:52Z [wt-cli]
Implemented the SAFE half: bulk field updates and reparent by EXPLICIT id-list only. `update` and `reparent` now take ids: Vec<String> (clap required, num_args=1..); same edit/parent applied per id via update_many/reparent_many, mirroring transition_many (per-id result line, all ids processed, exit non-zero if any failed). herd.update stays single-id; main.rs loops. TaskEdit derives Clone. Single-id usage preserved. Added tests/cli.rs::update_bulk_and_partial_failure. cargo test: 224 unit + 23 cli pass. Verified on release binary: bulk relabel both; partial failure (good id applied, missing reported to stderr, exit 1); bulk reparent under a parent; reparent partial failure. DEFERRED: filter/selector-driven mutation is intentionally NOT built here — that needs the human design decision tracked in yaks-7cc8. Scope: src/main.rs, src/herd.rs, tests/cli.rs.
