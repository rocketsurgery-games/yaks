---
id: yaks-6e21
title: 'Phase 1: full CLI write path + parity'
type: task
priority: 2
created: '2026-08-20T03:25:50Z'
updated: '2026-08-20T20:41:40Z'
labels:
- rust
- phase1
---

Port the whole CLI core (model save, create/update, status moves, deps/reparent, filtering, rollup, index) behind clap; achieve --json byte-parity with the Python tool. Goal: the Rust binary can fully manage a herd (dogfood on this repo).

---
▸ 2026-08-20T20:41:40Z
Phase 1 complete. Delivered with parity + tests: full write path (create/update/--note/--title, status moves, dep, reparent), query surface (list/next/tangled/search/stats/show, filtering), rollup, --json everywhere, golden snapshot harness (35 tests), npm distribution launcher, startup bench (~6ms vs ~45ms), and a schema-version gate. Deferred to top-level: yaks-8f50 (rkyv index — premature at current scale) and yaks-a49c (attach/detach). Dropped yaks-35b6 (CI interop test) — will iterate Rust-side and retire Python once comfortable.
