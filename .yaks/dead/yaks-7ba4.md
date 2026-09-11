---
id: yaks-7ba4
title: 'yaks verify: optional cached last-green short-circuit'
type: idea
priority: 3
created: '2026-09-11T00:11:13Z'
updated: '2026-09-11T01:25:04Z'
labels:
- meta
---

In a tight edit/verify loop -- and when a lane runs cargo test manually then immediately 'yaks verify' -- verify recompiles and reruns the full command even though nothing changed. An OPT-IN cache keyed on the verify command + a repo content hash (git tree + dirty state) could short-circuit to the recorded last-green, saving redundant multi-second/minute runs. Surfaced by lane-b during the parallel UI dogfood run. Keep it explicit/opt-in so verify stays trustworthy as evidence.

---
▸ 2026-09-11T01:06:32Z [coordinator]
Design analysis before building. VALUE: skip a redundant recompile+test in a tight loop. RISK (high): a wrong short-circuit reports PASS when code actually changed -- which corrupts verify's whole purpose as trustworthy evidence, and doctor --strict leans on it. So any cache MUST be (1) opt-in (e.g. 'yaks verify --if-unchanged'), never default; (2) conservative -- on ANY doubt, re-run; (3) a per-user REBUILDABLE cache (invariant: never committed, never a second source of truth), like cache.rs. OPEN QUESTION -- the change-detection key: (a) git HEAD sha + working-tree dirty hash (cheap/precise, but couples verify to git; yaks is filesystem-native, only optionally git-aware); (b) content hash of the tracked source tree (git-free but expensive); (c) mtimes (fast, unreliable across checkouts). Lean: (a), gated on git availability, falling back to always-run. Net: marginal time payoff vs. real correctness surface on the evidence mechanism.

---
▸ 2026-09-11T01:06:32Z [coordinator]
Build now, defer, or drop? Coordinator lean: DEFER (or slaughter) -- the trustworthiness cost (a stale cache falsely reporting PASS) outweighs the few seconds saved; if we do build it, it must be opt-in + conservative + git-keyed with an always-run fallback. Your call.

---
▸ 2026-09-11T01:15:37Z [Joel Webber]
Agreed, let's drop it. Err on the side of simplicity, and verification should be idempotent anyway.

---
▸ 2026-09-11T01:25:04Z [coordinator]
Dropped per Joel: err on the side of simplicity; verification should be idempotent anyway. Slaughtered.
