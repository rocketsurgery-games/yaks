---
id: yaks-7ba4
title: 'yaks verify: optional cached last-green short-circuit'
type: idea
priority: 3
created: '2026-09-11T00:11:13Z'
updated: '2026-09-11T00:11:13Z'
labels:
- meta
---

In a tight edit/verify loop -- and when a lane runs cargo test manually then immediately 'yaks verify' -- verify recompiles and reruns the full command even though nothing changed. An OPT-IN cache keyed on the verify command + a repo content hash (git tree + dirty state) could short-circuit to the recorded last-green, saving redundant multi-second/minute runs. Surfaced by lane-b during the parallel UI dogfood run. Keep it explicit/opt-in so verify stays trustworthy as evidence.
