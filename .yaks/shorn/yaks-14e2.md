---
id: yaks-14e2
title: Dependencies + reparent
type: task
priority: 3
created: '2026-08-20T03:26:09Z'
updated: '2026-08-20T19:13:25Z'
parent: yaks-6e21
depends_on:
- yaks-6b8c
labels:
- rust
---

---
▸ 2026-08-20T19:09:00Z
Starting. Porting dep add/remove (with existence checks + cycle rejection via depends_on_transitively), reparent (parent field rewrite; attach = set parent, detach = clear), matching yaklib.commands + yaklib.reparent. Will add unit tests for cycle detection and reparent.

---
▸ 2026-08-20T19:13:25Z
Done. store: add_dep/remove_dep (existence checks; Python-exact messages; no CLI cycle guard, matching yaklib) + reparent (self/missing/descendant/no-op guards via filter::descendant_ids). main: dep add|remove and reparent --parent/--unparent, byte-exact messages/exit codes. 4 graph tests (15 total). Live-verified on the herd: dep add/remove + reparent round-trips match Python and interoperate (Python reads Rust-written parent/deps). attach/detach split into its own yak (needs arboard + artifact-link handling).
