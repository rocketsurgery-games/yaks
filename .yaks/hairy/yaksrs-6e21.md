---
id: yaksrs-6e21
title: 'Phase 1: full CLI write path + parity'
type: task
priority: 2
created: '2026-08-20T03:25:50Z'
updated: '2026-08-20T03:25:50Z'
labels:
- rust
- phase1
---

Port the whole CLI core (model save, create/update, status moves, deps/reparent, filtering, rollup, index) behind clap; achieve --json byte-parity with the Python tool. Goal: the Rust binary can fully manage a herd (dogfood on this repo).
